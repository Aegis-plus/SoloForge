//! The engine: one open world, shared by the Tauri commands, the MCP table
//! server (the GM's hands) and the background image/GM tasks.

use std::sync::atomic::{AtomicBool, AtomicU16};
use std::sync::{Arc, Mutex};

use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::rules::checks::{self, Adv, CheckKind};
use crate::rules::{chargen, combat, dice, oracle, signed};
use crate::settings::Settings;
use crate::world::*;

/// Where engine events go (Tauri `emit`, a test recorder, or nowhere).
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: serde_json::Value);
}
pub struct NullSink;
impl EventSink for NullSink {
    fn emit(&self, _: &str, _: serde_json::Value) {}
}

pub struct Engine {
    pub store: Store,
    world: Mutex<Option<World>>,
    settings: Mutex<Settings>,
    sink: Arc<dyn EventSink>,
    rng: Mutex<StdRng>,
    pub mcp_port: AtomicU16,
    pub(crate) gm_busy: AtomicBool,
    pub(crate) gm_cancel: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub(crate) gm_queue: Mutex<Vec<String>>,
    /// The `--mcp-config` JSON for Claude Code (set once the MCP server is up).
    pub mcp_config: Mutex<Option<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorld {
    pub name: String,
    #[serde(default)]
    pub pitch: String,
    #[serde(default)]
    pub tone: Vec<String>,
    #[serde(default)]
    pub rules_dial: String,
    pub hero: chargen::HeroSpec,
    #[serde(default)]
    pub companion: bool,
}

pub type EResult<T> = Result<T, String>;

impl Engine {
    pub fn new(store: Store, settings: Settings, sink: Arc<dyn EventSink>) -> Arc<Engine> {
        Arc::new(Engine {
            store,
            world: Mutex::new(None),
            settings: Mutex::new(settings),
            sink,
            rng: Mutex::new(StdRng::from_entropy()),
            mcp_port: AtomicU16::new(0),
            gm_busy: AtomicBool::new(false),
            gm_cancel: Mutex::new(None),
            gm_queue: Mutex::new(vec![]),
            mcp_config: Mutex::new(None),
        })
    }

    /// Deterministic RNG for tests.
    pub fn seed(&self, seed: u64) {
        *self.rng.lock().unwrap() = StdRng::seed_from_u64(seed);
    }

    pub fn emit(&self, event: &str, payload: serde_json::Value) {
        self.sink.emit(event, payload);
    }

    pub fn with_rng<T>(&self, f: impl FnOnce(&mut StdRng) -> T) -> T {
        f(&mut self.rng.lock().unwrap())
    }

    // ---------- settings ----------
    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
    pub fn save_settings(&self, mut s: Settings) -> EResult<Settings> {
        {
            let cur = self.settings.lock().unwrap();
            if s.image.api_key.starts_with('•') {
                s.image.api_key = cur.image.api_key.clone();
            }
        }
        s.save().map_err(|e| format!("Couldn’t save settings: {e}. Check that the config folder is writable."))?;
        *self.settings.lock().unwrap() = s.clone();
        Ok(s.redacted())
    }

    // ---------- worlds ----------
    pub fn list_worlds(&self) -> Vec<WorldSummary> {
        self.store.list()
    }

    pub fn world(&self) -> Option<World> {
        self.world.lock().unwrap().clone()
    }

    pub fn world_id(&self) -> Option<String> {
        self.world.lock().unwrap().as_ref().map(|w| w.id.clone())
    }

    pub fn open_world(&self, id: &str) -> EResult<World> {
        let mut w = self.store.load(id).map_err(|e| format!("Couldn’t open that world: {e}"))?;
        w.session += 1;
        w.push_chat(ChatKind::System { text: format!("Session {} begins.", w.session) });
        self.store.save(&w).map_err(|e| e.to_string())?;
        *self.world.lock().unwrap() = Some(w.clone());
        self.emit("world://changed", json!(w));
        Ok(w)
    }

    pub fn close_world(&self) {
        *self.world.lock().unwrap() = None;
    }

    pub fn create_world(&self, spec: NewWorld) -> EResult<World> {
        if spec.name.trim().is_empty() {
            return Err("Give your world a name to continue, e.g. “The Drowned Marches”.".into());
        }
        let hero = chargen::build_hero(&spec.hero)?;
        let mut w = World::new(spec.name.trim(), &spec.pitch);
        w.tone = spec.tone;
        if !spec.rules_dial.is_empty() {
            w.rules_dial = spec.rules_dial;
        }
        let mut scene = Scene::new("Session Zero", 20, 14);
        scene.description = "An open grid while the GM sets the first scene.".into();
        let (cx, cy) = (scene.width / 2, scene.height / 2);
        scene.tokens.push(Token { id: new_id(), actor_id: hero.id.clone(), x: cx, y: cy, hidden: false });
        scene.reveal_radius(cx, cy, 6);
        w.active_scene = Some(scene.id.clone());
        w.scenes.push(scene);
        if spec.companion {
            w.note_event("The player asked for a GM-run companion. Create one with create_actor (kind companion) in session zero.");
        }
        w.actors.push(hero);
        w.push_chat(ChatKind::System { text: "World created. Send a message, or press Begin Session Zero, and the GM sets the first scene.".into() });
        self.store.save(&w).map_err(|e| e.to_string())?;
        *self.world.lock().unwrap() = Some(w.clone());
        self.emit("world://changed", json!(w));
        Ok(w)
    }

    pub fn delete_world(&self, id: &str) -> EResult<()> {
        if self.world_id().as_deref() == Some(id) {
            self.close_world();
        }
        self.store.trash(id).map_err(|e| format!("Couldn’t delete that world: {e}"))
    }
    pub fn restore_world(&self, id: &str) -> EResult<()> {
        self.store.restore(id).map_err(|e| format!("Couldn’t restore that world: {e}"))
    }

    /// Mutate the open world, then save and broadcast it.
    pub fn with_world<T>(&self, f: impl FnOnce(&mut World) -> EResult<T>) -> EResult<T> {
        let mut guard = self.world.lock().unwrap();
        let w = guard.as_mut().ok_or_else(|| "No world is open. Open one from Your Worlds.".to_string())?;
        let out = f(w)?;
        w.touch();
        if let Err(e) = self.store.save(w) {
            eprintln!("soloforge: save failed: {e}");
        }
        self.emit("world://changed", json!(w));
        Ok(out)
    }

    pub fn images_dir(&self) -> Option<std::path::PathBuf> {
        self.world_id().map(|id| self.store.images_dir(&id))
    }

    // ---------- player actions (Tauri commands) ----------

    pub fn player_roll(&self, expr: &str, label: &str) -> EResult<RollCard> {
        let r = self.with_rng(|rng| dice::roll(expr, rng)).map_err(|e| format!("{e}. Try something like 2d6+3."))?;
        self.with_world(|w| {
            let who = w.pc().map(|p| p.name.clone()).unwrap_or_default();
            let title = if label.trim().is_empty() { format!("Roll {expr}") } else { label.to_string() };
            let card = RollCard { title: title.clone(), actor: who.clone(), lines: vec![RollLine { label: title.clone(), detail: r.breakdown(), total: r.total, kind: "plain".into() }], outcome: "none".into(), note: String::new(), by_gm: false };
            w.push_chat(ChatKind::Roll { card: card.clone() });
            w.note_event(format!("Player rolled {title}: {} = {}", r.breakdown(), r.total));
            Ok(card)
        })
    }

    pub fn player_check(&self, kind: &str, adv: &str) -> EResult<RollCard> {
        let k = CheckKind::parse(kind).ok_or_else(|| format!("“{kind}” isn’t a skill, ability or save."))?;
        self.with_world(|w| {
            let pc = w.pc().ok_or("Your world has no hero yet.")?.clone();
            let res = self.with_rng(|rng| checks::check(&pc, k, None, Adv::parse(adv), rng));
            w.push_chat(ChatKind::Roll { card: res.card.clone() });
            w.note_event(format!("Player rolled {} for {}: {} (natural {}). The GM decides the DC and outcome.", k.label(), pc.name, res.total, res.natural));
            Ok(res.card)
        })
    }

    pub fn player_attack(&self, attack: &str, target: &str, adv: &str) -> EResult<RollCard> {
        self.with_world(|w| {
            let pc = w.pc().ok_or("Your world has no hero yet.")?.clone();
            let atk = pc.attacks.iter().find(|a| a.name.eq_ignore_ascii_case(attack)).ok_or_else(|| format!("{} has no attack called {attack}.", pc.name))?.clone();
            let tgt = w.actor(target).ok_or_else(|| format!("There’s no “{target}” on the table. Pick a target token first."))?.clone();
            let o = self.with_rng(|rng| combat::resolve_attack(&pc, &atk, &tgt, Adv::parse(adv), rng));
            let mut card = o.card.clone();
            if o.hit {
                let t = w.actor_mut(&tgt.id).unwrap();
                let (b, a) = combat::apply_damage(t, o.damage);
                card.note = format!("{}: {b} → {a} HP{}", tgt.name, if a == 0 { " · Down" } else { "" });
            }
            w.push_chat(ChatKind::Roll { card: card.clone() });
            w.note_event(format!(
                "Player attacked {} with {}: {} ({}). {}",
                tgt.name,
                atk.name,
                if o.crit { "critical hit" } else if o.hit { "hit" } else { "miss" },
                card.lines.iter().map(|l| format!("{} {}", l.label, l.total)).collect::<Vec<_>>().join(", "),
                card.note
            ));
            Ok(card)
        })
    }

    pub fn move_token(&self, token_id: &str, x: i32, y: i32) -> EResult<()> {
        self.with_world(|w| {
            let pc_id = w.pc().map(|p| p.id.clone());
            let sc = w.scene_mut().ok_or("There’s no active scene.")?;
            if !sc.in_bounds(x, y) {
                return Err("That square is off the map.".into());
            }
            let t = sc.tokens.iter_mut().find(|t| t.id == token_id).ok_or("That token isn’t on this map.")?;
            if Some(&t.actor_id) != pc_id.as_ref() {
                return Err("You can only move your own token. Ask the GM to move others.".into());
            }
            let ft = grid_distance_ft(t.x, t.y, x, y);
            t.x = x;
            t.y = y;
            sc.reveal_radius(x, y, 6);
            let name = w.pc().map(|p| p.name.clone()).unwrap_or_default();
            w.note_event(format!("Player moved {name} {ft} ft to ({x}, {y})."));
            Ok(())
        })
    }

    pub fn end_turn(&self) -> EResult<()> {
        self.with_world(|w| {
            let next = combat::next_turn(w).ok_or("No combat is running.")?;
            let n = w.actor(&next).map(|a| a.name.clone()).unwrap_or_default();
            let round = w.combat.as_ref().map(|c| c.round).unwrap_or(1);
            w.note_event(format!("Player ended their turn. It is {n}'s turn, round {round}."));
            Ok(())
        })
    }

    pub fn ask_oracle(&self, question: &str, likelihood: &str) -> EResult<()> {
        self.with_world(|w| {
            let l = oracle::Likelihood::parse(likelihood);
            let a = self.with_rng(|rng| oracle::ask(l, w.tension, rng));
            w.push_chat(ChatKind::Oracle { question: question.into(), likelihood: l.label().into(), roll: a.roll, answer: a.answer.clone(), event: a.event.clone() });
            w.note_event(format!("Player asked the oracle “{question}” ({}): d100 {} → {}{}", l.label(), a.roll, a.answer, a.event.map(|e| format!("; random event: {e}")).unwrap_or_default()));
            Ok(())
        })
    }

    pub fn resolve_proposal(&self, id: &str, accept: bool) -> EResult<()> {
        self.with_world(|w| {
            let idx = w.proposals.iter().position(|p| p.id == id).ok_or("That proposal is gone.")?;
            let p = w.proposals.remove(idx);
            if accept {
                let a = w.actor_mut(&p.actor_id).ok_or("That character is gone.")?;
                for c in &p.changes {
                    apply_sheet_change(a, c);
                }
                w.note_event(format!("Player accepted the sheet changes: {}", p.reason));
            } else {
                w.note_event(format!("Player declined the sheet changes: {}", p.reason));
            }
            Ok(())
        })
    }

    pub fn rest(&self, long: bool) -> EResult<String> {
        self.with_world(|w| {
            let a = w.pc_mut().ok_or("Your world has no hero yet.")?;
            let msg = if long {
                a.hp = a.max_hp;
                a.spell_slots = a.spell_slots_max.clone();
                a.fate = 3;
                a.hit_dice = a.level;
                a.conditions.retain(|c| c != "Down");
                format!("{} takes a long rest: full HP, spell slots and 3 Fate.", a.name)
            } else {
                let heal = (a.max_hp / 4).max(1);
                let (b, af) = combat::heal(a, heal);
                format!("{} takes a short rest: {b} → {af} HP.", a.name)
            };
            w.push_chat(ChatKind::System { text: msg.clone() });
            w.note_event(msg.clone());
            Ok(msg)
        })
    }

    pub fn set_story_mode(&self, on: bool) -> EResult<()> {
        self.with_world(|w| {
            w.story_mode = on;
            Ok(())
        })
    }

    pub fn add_note(&self, title: &str, body: &str) -> EResult<()> {
        self.with_world(|w| {
            w.journal.push(JournalEntry { id: new_id(), title: title.into(), category: "note".into(), body: body.into(), image: None, objectives: vec![], active: false, updated_at: now_ms(), by_player: true });
            Ok(())
        })
    }

    pub fn update_notes(&self, notes: &str) -> EResult<()> {
        self.with_world(|w| {
            if let Some(p) = w.pc_mut() {
                p.notes = notes.into();
            }
            Ok(())
        })
    }
}

pub fn apply_sheet_change(a: &mut Actor, c: &SheetChange) {
    match c {
        SheetChange::AddItem { name, qty, note } => {
            if let Some(it) = a.inventory.iter_mut().find(|i| i.name.eq_ignore_ascii_case(name)) {
                it.qty += qty;
            } else {
                a.inventory.push(Item { name: name.clone(), qty: *qty, note: note.clone() });
            }
        }
        SheetChange::RemoveItem { name, qty } => {
            if let Some(i) = a.inventory.iter().position(|i| i.name.eq_ignore_ascii_case(name)) {
                if a.inventory[i].qty <= *qty {
                    a.inventory.remove(i);
                } else {
                    a.inventory[i].qty -= qty;
                }
            }
        }
        SheetChange::Xp { amount } => a.xp += amount,
        SheetChange::Gold { amount } => a.gold = (a.gold + amount).max(0),
        SheetChange::Condition { name, add } => {
            if *add {
                if !a.conditions.contains(name) {
                    a.conditions.push(name.clone());
                }
            } else {
                a.conditions.retain(|c| c != name);
            }
        }
    }
}

pub fn describe_change(c: &SheetChange) -> String {
    match c {
        SheetChange::AddItem { name, qty, .. } => format!("+ {name}{}", if *qty > 1 { format!(" ×{qty}") } else { String::new() }),
        SheetChange::RemoveItem { name, qty } => format!("− {qty} {name}"),
        SheetChange::Xp { amount } => format!("+ {amount} XP"),
        SheetChange::Gold { amount } => format!("{} gp", signed(*amount)),
        SheetChange::Condition { name, add } => format!("{} {name}", if *add { "Gains" } else { "Loses" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn engine() -> Arc<Engine> {
        let dir = std::env::temp_dir().join(format!("sf-engine-{}", new_id()));
        let e = Engine::new(Store::new(dir), Settings::default(), Arc::new(NullSink));
        e.seed(4);
        e
    }

    pub fn new_world(e: &Engine) -> World {
        e.create_world(NewWorld {
            name: "The Ashen Vale".into(),
            pitch: "A valley that burned".into(),
            tone: vec!["Gritty".into()],
            rules_dial: "balanced".into(),
            hero: chargen::HeroSpec { name: "Aria Thorne".into(), ancestry: "Wood Elf".into(), class: "Ranger".into(), scores: Abilities { str: 10, dex: 15, con: 14, int: 10, wis: 13, cha: 8 }, appearance: String::new(), background: String::new() },
            companion: false,
        })
        .unwrap()
    }

    #[test]
    fn create_world_places_the_hero() {
        let e = engine();
        let w = new_world(&e);
        let sc = w.scene().unwrap();
        assert_eq!(sc.tokens.len(), 1);
        assert!(sc.revealed.iter().any(|r| *r));
        assert_eq!(e.list_worlds().len(), 1);
        assert!(e.create_world(NewWorld { name: " ".into(), ..serde_json::from_value(json!({"name":"x","hero":{"name":"a","ancestry":"Human","class":"Fighter","scores":{"str":8,"dex":8,"con":8,"int":8,"wis":8,"cha":8}}})).unwrap() }).is_err());
    }

    #[test]
    fn player_can_only_move_own_token_on_the_map() {
        let e = engine();
        let w = new_world(&e);
        let tok = w.scene().unwrap().tokens[0].id.clone();
        e.move_token(&tok, 2, 3).unwrap();
        assert!(e.move_token(&tok, 99, 3).is_err());
        let w = e.world().unwrap();
        assert_eq!((w.scene().unwrap().tokens[0].x, w.scene().unwrap().tokens[0].y), (2, 3));
        assert!(w.pending_events.last().unwrap().contains("to (2, 3)"));
    }

    #[test]
    fn checks_and_rolls_land_in_chat_and_events() {
        let e = engine();
        new_world(&e);
        e.player_check("stealth", "adv").unwrap();
        e.player_roll("2d6+3", "Damage").unwrap();
        assert!(e.player_roll("2d6+", "x").is_err());
        let w = e.world().unwrap();
        assert_eq!(w.chat.iter().filter(|c| matches!(c.kind, ChatKind::Roll { .. })).count(), 2);
        assert!(w.pending_events.iter().any(|p| p.contains("Stealth Check")));
    }

    #[test]
    fn proposals_apply_only_when_accepted() {
        let e = engine();
        let w = new_world(&e);
        let pc = w.pc().unwrap().id.clone();
        e.with_world(|w| {
            w.proposals.push(SheetProposal { id: "p1".into(), actor_id: pc.clone(), reason: "loot".into(), changes: vec![SheetChange::AddItem { name: "Silver bell".into(), qty: 1, note: String::new() }, SheetChange::Xp { amount: 50 }] });
            w.proposals.push(SheetProposal { id: "p2".into(), actor_id: pc.clone(), reason: "nope".into(), changes: vec![SheetChange::Xp { amount: 999 }] });
            Ok(())
        })
        .unwrap();
        e.resolve_proposal("p1", true).unwrap();
        e.resolve_proposal("p2", false).unwrap();
        let w = e.world().unwrap();
        let a = w.pc().unwrap();
        assert_eq!(a.xp, 50);
        assert!(a.inventory.iter().any(|i| i.name == "Silver bell"));
        assert!(w.proposals.is_empty());
    }
}

