//! The GM's hands: every MCP tool the Claude Code GM can call.
//! Each tool changes the world through the engine (so the UI updates live)
//! and returns a short text result for the GM.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::art::{self, ImageKind, ImageRequest};
use crate::engine::{apply_sheet_change, describe_change, Engine, EResult};
use crate::rules::checks::{self, Adv, CheckKind};
use crate::rules::{combat, compendium, dice, oracle, signed};
use crate::world::*;

const RULES: &str = include_str!("../../../../assets/rules/solo5e.md");

fn s<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty())
}
fn req<'a>(a: &'a Value, k: &str) -> EResult<&'a str> {
    s(a, k).ok_or_else(|| format!("missing required argument `{k}`"))
}
fn i(a: &Value, k: &str) -> Option<i64> {
    a.get(k).and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
}
fn b(a: &Value, k: &str) -> Option<bool> {
    a.get(k).and_then(|v| v.as_bool())
}
fn strs(a: &Value, k: &str) -> Vec<String> {
    a.get(k).and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn prop(t: &str, d: &str) -> Value {
    json!({"type": t, "description": d})
}
fn tool(name: &str, desc: &str, props: Value, required: &[&str]) -> Value {
    json!({"name": name, "description": desc, "inputSchema": {"type":"object","properties": props, "required": required}})
}

pub fn definitions() -> Vec<Value> {
    let arr_s = |d: &str| json!({"type":"array","items":{"type":"string"},"description": d});
    vec![
        tool("get_state", "Read the table: world, hero sheet, active scene with tokens and positions, combat order, journal, and the table events since your last turn. Call this first every turn.", json!({}), &[]),
        tool("get_actor", "Full sheet of one character (by name or id).", json!({"actor": prop("string","name or id")}), &["actor"]),
        tool("roll", "Roll dice with the table's RNG, e.g. '2d6+3', '1d20 adv', '4d6kh3'. Shown in chat unless secret.", json!({"expr": prop("string","dice expression"), "label": prop("string","what the roll is for"), "secret": prop("boolean","hide the result from the player")}), &["expr"]),
        tool("check", "Ability check, skill check or saving throw for any character, e.g. check='stealth', 'dex save', 'wisdom'.", json!({"actor": prop("string","name or id"), "check": prop("string","skill, ability, or '<ability> save'"), "dc": prop("integer","difficulty class"), "adv": prop("string","advantage | disadvantage | normal"), "secret": prop("boolean","hide from the player")}), &["actor","check"]),
        tool("attack", "Resolve an attack: rolls to hit against the target's AC and damage, and applies the damage.", json!({"attacker": prop("string","name or id"), "target": prop("string","name or id"), "attack": prop("string","attack name from the attacker's sheet; defaults to the first"), "adv": prop("string","advantage | disadvantage | normal")}), &["attacker","target"]),
        tool("apply_damage", "Apply damage (temp HP first; minions drop on any hit).", json!({"actor": prop("string","name or id"), "amount": prop("integer","damage"), "type": prop("string","damage type")}), &["actor","amount"]),
        tool("heal", "Heal a character.", json!({"actor": prop("string","name or id"), "amount": prop("integer","HP")}), &["actor","amount"]),
        tool("set_condition", "Add or remove a condition (Poisoned, Prone, Frightened…).", json!({"actor": prop("string","name or id"), "condition": prop("string","condition"), "add": prop("boolean","true to add, false to remove")}), &["actor","condition"]),
        tool("rest", "The hero takes a short or long rest.", json!({"kind": prop("string","short | long")}), &["kind"]),
        tool("award_xp", "Award XP to the hero (becomes a proposal the player accepts).", json!({"amount": prop("integer","XP"), "reason": prop("string","why")}), &["amount"]),
        tool("give_item", "Give the hero (or anyone) an item. For the hero this becomes a proposal the player accepts.", json!({"actor": prop("string","defaults to the hero"), "name": prop("string","item"), "qty": prop("integer","quantity"), "note": prop("string","short note")}), &["name"]),
        tool("remove_item", "Remove an item (ammo, consumed potion…).", json!({"actor": prop("string","defaults to the hero"), "name": prop("string","item"), "qty": prop("integer","quantity")}), &["name"]),
        tool("rules_lookup", "Look up the Solo 5e-inspired rules: checks, combat, conditions, rests, solo rules, encounter, oracle, levelling.", json!({"topic": prop("string","topic")}), &["topic"]),
        tool("oracle", "Ask the solo oracle a yes/no question about the world. Doubles under Tension can trigger a random event.", json!({"question": prop("string","yes/no question"), "likelihood": prop("string","impossible | unlikely | even odds | likely | certain")}), &["question"]),
        tool("tension", "Raise or lower Tension (1–9).", json!({"delta": prop("integer","+1 or -1"), "reason": prop("string","why")}), &["delta"]),
        tool("compendium_search", "Search built-in monsters, items and spells.", json!({"query": prop("string","search text")}), &["query"]),
        tool("create_actor", "Create an NPC, monster or companion — from the compendium (compendium='goblin') or a custom stat block.", json!({
            "name": prop("string","display name, e.g. 'Goblin B' or 'Mother Ilse'"),
            "compendium": prop("string","compendium monster id or name to copy"),
            "kind": prop("string","monster | npc | companion"),
            "disposition": prop("string","hostile | neutral | friendly"),
            "ac": prop("integer","armour class"), "hp": prop("integer","hit points"),
            "abilities": json!({"type":"array","items":{"type":"integer"},"description":"[STR, DEX, CON, INT, WIS, CHA]"}),
            "attacks": json!({"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"to_hit":{"type":"integer"},"damage":{"type":"string"},"damage_type":{"type":"string"}}}}),
            "minion": prop("boolean","drops on any hit"), "appearance": prop("string","for portraits"), "notes": prop("string","GM notes")
        }), &["name"]),
        tool("update_actor", "Change fields on a character: ac, max_hp, hp, speed, disposition, notes, appearance, name.", json!({"actor": prop("string","name or id"), "fields": json!({"type":"object","description":"fields to change"})}), &["actor","fields"]),
        tool("create_scene", "Create and activate a new battle-map scene. The hero's token is not moved automatically: use place_token.", json!({"name": prop("string","scene name"), "description": prop("string","what's here"), "width": prop("integer","columns, 8–40"), "height": prop("integer","rows, 8–30"), "generate_map": prop("boolean","paint a top-down map with the image endpoint"), "map_prompt": prop("string","what the map shows")}), &["name"]),
        tool("activate_scene", "Switch to an existing scene by name or id.", json!({"scene": prop("string","name or id")}), &["scene"]),
        tool("place_token", "Put a character's token on the active map (or move it there).", json!({"actor": prop("string","name or id"), "x": prop("integer","column"), "y": prop("integer","row"), "hidden": prop("boolean","hidden from the player")}), &["actor","x","y"]),
        tool("move_token", "Move a token on the active map.", json!({"actor": prop("string","name or id"), "x": prop("integer","column"), "y": prop("integer","row")}), &["actor","x","y"]),
        tool("remove_token", "Remove a token from the active map.", json!({"actor": prop("string","name or id")}), &["actor"]),
        tool("reveal_fog", "Reveal map cells to the player: a rectangle, or all=true.", json!({"x": prop("integer","left"), "y": prop("integer","top"), "w": prop("integer","width"), "h": prop("integer","height"), "all": prop("boolean","reveal the whole map")}), &[]),
        tool("start_combat", "Start combat: rolls initiative for every listed character (include the hero).", json!({"actors": arr_s("names or ids"), "hidden": arr_s("names to keep hidden in the initiative bar"), "name": prop("string","encounter name")}), &["actors"]),
        tool("next_turn", "Advance initiative to the next standing combatant.", json!({}), &[]),
        tool("end_combat", "End combat.", json!({"summary": prop("string","one line")}), &[]),
        tool("journal_write", "Create a journal entry: quest, npc, lore or handout.", json!({"title": prop("string","Title Case"), "category": prop("string","quest | npc | lore | handout"), "body": prop("string","1–3 short paragraphs"), "objectives": arr_s("quest objectives"), "active": prop("boolean","the current quest")}), &["title","category","body"]),
        tool("journal_update", "Update a journal entry: new body, add/complete objectives.", json!({"title": prop("string","entry title or id"), "body": prop("string","replacement body"), "add_objectives": arr_s("new objectives"), "complete": arr_s("objective texts to tick"), "active": prop("boolean","")}), &["title"]),
        tool("generate_image", "Ask the image endpoint for art. kind=map (active scene background), token (portrait for actor), art (scene art in chat). Returns at once; the image appears when ready.", json!({"kind": prop("string","map | token | art"), "prompt": prop("string","what to show"), "actor": prop("string","for kind=token")}), &["kind","prompt"]),
        tool("story_mode", "Switch Story Mode (map hidden, big narration, choices) on or off.", json!({"on": prop("boolean","on/off"), "choices": arr_s("2–4 choices in Title Case")}), &["on"]),
        tool("suggest_actions", "Offer 2–4 short quick actions under the chat input (Title Case).", json!({"actions": arr_s("actions")}), &["actions"]),
    ]
}

fn find_id(w: &World, q: &str) -> EResult<String> {
    w.actor(q).map(|a| a.id.clone()).ok_or_else(|| {
        let names: Vec<&str> = w.actors.iter().map(|a| a.name.as_str()).collect();
        format!("no character named “{q}”. Known: {}", names.join(", "))
    })
}

fn tool_line(w: &mut World, text: impl Into<String>) {
    w.push_chat(ChatKind::Tool { text: text.into() });
}

pub fn call(engine: &Arc<Engine>, name: &str, a: &Value) -> EResult<String> {
    let e = engine;
    match name {
        "get_state" => {
            let w = e.world().ok_or("No world is open.")?;
            Ok(state_json(&w, e.settings().image.enabled()).to_string())
        }
        "get_actor" => {
            let w = e.world().ok_or("No world is open.")?;
            let actor = w.actor(req(a, "actor")?).ok_or("no such character")?;
            Ok(serde_json::to_string(actor).unwrap_or_default())
        }
        "roll" => {
            let expr = req(a, "expr")?;
            let label = s(a, "label").unwrap_or("GM roll");
            let secret = b(a, "secret").unwrap_or(false);
            let r = e.with_rng(|rng| dice::roll(expr, rng)).map_err(|x| x.to_string())?;
            e.with_world(|w| {
                if secret {
                    tool_line(w, format!("GM rolled secretly: {label}"));
                } else {
                    let card = RollCard { title: label.into(), actor: "GM".into(), lines: vec![RollLine { label: label.into(), detail: r.breakdown(), total: r.total, kind: "plain".into() }], outcome: "none".into(), note: String::new(), by_gm: true };
                    w.push_chat(ChatKind::Roll { card });
                }
                Ok(())
            })?;
            Ok(format!("{label}: {} = {}", r.breakdown(), r.total))
        }
        "check" => {
            let kind = CheckKind::parse(req(a, "check")?).ok_or("unknown check; use a skill, ability, or '<ability> save'")?;
            let dc = i(a, "dc").map(|x| x as i32);
            let adv = Adv::parse(s(a, "adv").unwrap_or(""));
            let secret = b(a, "secret").unwrap_or(false);
            e.with_world(|w| {
                let id = find_id(w, req(a, "actor")?)?;
                let actor = w.actor(&id).unwrap().clone();
                let r = e.with_rng(|rng| checks::check(&actor, kind, dc, adv, rng));
                if secret {
                    tool_line(w, format!("GM rolled {} for {} secretly", kind.label(), actor.name));
                } else {
                    let mut card = r.card.clone();
                    card.by_gm = true;
                    w.push_chat(ChatKind::Roll { card });
                }
                let verdict = match r.success {
                    Some(true) => " — success",
                    Some(false) => " — failure",
                    None => "",
                };
                Ok(format!("{} {}: {} (natural {}){}", actor.name, kind.label(), r.total, r.natural, verdict))
            })
        }
        "attack" => e.with_world(|w| {
            let att_id = find_id(w, req(a, "attacker")?)?;
            let tgt_id = find_id(w, req(a, "target")?)?;
            let attacker = w.actor(&att_id).unwrap().clone();
            let target = w.actor(&tgt_id).unwrap().clone();
            let atk = match s(a, "attack") {
                Some(n) => attacker.attacks.iter().find(|x| x.name.eq_ignore_ascii_case(n)).cloned(),
                None => attacker.attacks.first().cloned(),
            }
            .ok_or_else(|| format!("{} has no such attack. Attacks: {}", attacker.name, attacker.attacks.iter().map(|x| x.name.clone()).collect::<Vec<_>>().join(", ")))?;
            let o = e.with_rng(|rng| combat::resolve_attack(&attacker, &atk, &target, Adv::parse(s(a, "adv").unwrap_or("")), rng));
            let mut card = o.card.clone();
            card.by_gm = true;
            let mut res = format!("{} attacks {} with {}: {}", attacker.name, target.name, atk.name, if o.crit { "CRITICAL HIT" } else if o.hit { "hit" } else { "miss" });
            if o.hit {
                let t = w.actor_mut(&tgt_id).unwrap();
                let (bf, af) = combat::apply_damage(t, o.damage);
                card.note = format!("{}: {bf} → {af} HP{}", target.name, if af == 0 { " · Down" } else { "" });
                res.push_str(&format!(", {} damage ({bf} → {af} HP){}", o.damage, if af == 0 { ", DOWN" } else { "" }));
            }
            w.push_chat(ChatKind::Roll { card });
            Ok(res)
        }),
        "apply_damage" | "heal" => {
            let amount = i(a, "amount").ok_or("missing amount")? as i32;
            e.with_world(|w| {
                let id = find_id(w, req(a, "actor")?)?;
                let t = w.actor_mut(&id).unwrap();
                let nm = t.name.clone();
                let (bf, af) = if name == "heal" { combat::heal(t, amount) } else { combat::apply_damage(t, amount) };
                let verb = if name == "heal" { "healed" } else { "took" };
                let ty = s(a, "type").map(|t| format!(" {t}")).unwrap_or_default();
                tool_line(w, format!("{nm} {verb} {amount}{ty}: {bf} → {af} HP"));
                Ok(format!("{nm}: {bf} → {af} HP"))
            })
        }
        "set_condition" => e.with_world(|w| {
            let id = find_id(w, req(a, "actor")?)?;
            let c = req(a, "condition")?.to_string();
            let add = b(a, "add").unwrap_or(true);
            let t = w.actor_mut(&id).unwrap();
            apply_sheet_change(t, &SheetChange::Condition { name: c.clone(), add });
            let nm = t.name.clone();
            tool_line(w, format!("{nm} {} {c}", if add { "is now" } else { "is no longer" }));
            Ok("ok".into())
        }),
        "rest" => e.rest(s(a, "kind").unwrap_or("short") == "long"),
        "award_xp" | "give_item" | "remove_item" => {
            let ask = e.settings().gm.ask_before_sheet_changes;
            e.with_world(|w| {
                let pc_id = w.pc().map(|p| p.id.clone()).ok_or("no hero")?;
                let target = match s(a, "actor") {
                    Some(q) => find_id(w, q)?,
                    None => pc_id.clone(),
                };
                let change = match name {
                    "award_xp" => SheetChange::Xp { amount: i(a, "amount").unwrap_or(0).max(0) as u32 },
                    "give_item" => SheetChange::AddItem { name: req(a, "name")?.into(), qty: i(a, "qty").unwrap_or(1).max(1) as u32, note: s(a, "note").unwrap_or("").into() },
                    _ => SheetChange::RemoveItem { name: req(a, "name")?.into(), qty: i(a, "qty").unwrap_or(1).max(1) as u32 },
                };
                let reason = s(a, "reason").unwrap_or(match name { "award_xp" => "experience", "give_item" => "loot", _ => "used up" }).to_string();
                // Consumables leaving the sheet never need a prompt; gains for the hero do when asked.
                if ask && target == pc_id && name != "remove_item" {
                    if let Some(p) = w.proposals.iter_mut().find(|p| p.actor_id == pc_id) {
                        p.changes.push(change.clone());
                        p.reason = format!("{}; {reason}", p.reason);
                    } else {
                        w.proposals.push(SheetProposal { id: new_id(), actor_id: pc_id, reason, changes: vec![change.clone()] });
                    }
                    Ok(format!("proposed to the player: {}", describe_change(&change)))
                } else {
                    let t = w.actor_mut(&target).unwrap();
                    apply_sheet_change(t, &change);
                    let nm = t.name.clone();
                    tool_line(w, format!("{nm}: {}", describe_change(&change)));
                    Ok("done".into())
                }
            })
        }
        "rules_lookup" => Ok(rules_section(req(a, "topic")?)),
        "oracle" => {
            let q = req(a, "question")?.to_string();
            let l = oracle::Likelihood::parse(s(a, "likelihood").unwrap_or("even odds"));
            e.with_world(|w| {
                let ans = e.with_rng(|rng| oracle::ask(l, w.tension, rng));
                w.push_chat(ChatKind::Oracle { question: q.clone(), likelihood: l.label().into(), roll: ans.roll, answer: ans.answer.clone(), event: ans.event.clone() });
                Ok(format!("Oracle ({}): d100 {} → {}{}", l.label(), ans.roll, ans.answer, ans.event.map(|x| format!(". RANDOM EVENT: {x}")).unwrap_or_default()))
            })
        }
        "tension" => e.with_world(|w| {
            let d = i(a, "delta").unwrap_or(0) as i32;
            w.tension = (w.tension as i32 + d).clamp(1, 9) as u8;
            Ok(format!("Tension is now {}", w.tension))
        }),
        "compendium_search" => {
            let r = compendium::search(req(a, "query")?);
            Ok(if r.is_empty() { "No matches. Use create_actor with a custom stat block.".into() } else { r.join("\n") })
        }
        "create_actor" => e.with_world(|w| {
            let mut actor = match s(a, "compendium") {
                Some(c) => compendium::monster(c).ok_or_else(|| format!("“{c}” isn’t in the compendium; omit `compendium` and give a stat block"))?,
                None => {
                    let ab = a.get("abilities").and_then(|v| v.as_array()).map(|v| v.iter().map(|x| x.as_i64().unwrap_or(10) as i32).collect::<Vec<_>>()).unwrap_or_default();
                    let g = |k: usize| *ab.get(k).unwrap_or(&10);
                    let mut m = compendium::monster("bandit").unwrap();
                    m.abilities = Abilities { str: g(0), dex: g(1), con: g(2), int: g(3), wis: g(4), cha: g(5) };
                    m.minion = false;
                    m.cr = String::new();
                    m.traits = vec![];
                    m.attacks = vec![];
                    m
                }
            };
            actor.name = req(a, "name")?.to_string();
            if let Some(k) = s(a, "kind") {
                actor.kind = match k { "npc" => ActorKind::Npc, "companion" => ActorKind::Companion, _ => ActorKind::Monster };
            }
            actor.disposition = match s(a, "disposition").unwrap_or(if actor.kind == ActorKind::Companion { "friendly" } else if actor.kind == ActorKind::Npc { "neutral" } else { "hostile" }) {
                "friendly" => Disposition::Friendly,
                "neutral" => Disposition::Neutral,
                _ => Disposition::Hostile,
            };
            if let Some(v) = i(a, "ac") { actor.ac = v as i32; }
            if let Some(v) = i(a, "hp") { actor.max_hp = v as i32; actor.hp = v as i32; }
            if let Some(v) = b(a, "minion") { actor.minion = v; }
            if let Some(v) = s(a, "appearance") { actor.appearance = v.into(); }
            if let Some(v) = s(a, "notes") { actor.notes = v.into(); }
            if let Some(atks) = a.get("attacks").and_then(|v| v.as_array()) {
                actor.attacks = atks.iter().filter_map(|x| serde_json::from_value::<Attack>(json!({"name": x.get("name")?, "to_hit": x.get("to_hit").cloned().unwrap_or(json!(3)), "damage": x.get("damage").cloned().unwrap_or(json!("1d6")), "damage_type": x.get("damage_type").cloned().unwrap_or(json!(""))})).ok()).collect();
            }
            if w.actors.iter().any(|x| x.name.eq_ignore_ascii_case(&actor.name)) {
                return Err(format!("a character named {} already exists; use a distinct name like '{} B'", actor.name, actor.name));
            }
            let summary = format!("{} (id {}): AC {}, HP {}, {:?}{}", actor.name, actor.id, actor.ac, actor.hp, actor.disposition, if actor.minion { ", minion" } else { "" });
            w.actors.push(actor);
            Ok(summary)
        }),
        "update_actor" => e.with_world(|w| {
            let id = find_id(w, req(a, "actor")?)?;
            let f = a.get("fields").cloned().unwrap_or(json!({}));
            let t = w.actor_mut(&id).unwrap();
            if let Some(v) = i(&f, "ac") { t.ac = v as i32; }
            if let Some(v) = i(&f, "max_hp") { t.max_hp = v as i32; }
            if let Some(v) = i(&f, "hp") { t.hp = (v as i32).clamp(0, t.max_hp); }
            if let Some(v) = i(&f, "speed") { t.speed = v as i32; }
            if let Some(v) = s(&f, "notes") { t.notes = v.into(); }
            if let Some(v) = s(&f, "appearance") { t.appearance = v.into(); }
            if let Some(v) = s(&f, "name") { t.name = v.into(); }
            if let Some(v) = s(&f, "disposition") {
                t.disposition = match v { "friendly" => Disposition::Friendly, "neutral" => Disposition::Neutral, _ => Disposition::Hostile };
            }
            Ok("updated".into())
        }),
        "create_scene" => {
            let gen = b(a, "generate_map").unwrap_or(false);
            let name = req(a, "name")?.to_string();
            let desc = s(a, "description").unwrap_or("").to_string();
            let (wd, ht) = (i(a, "width").unwrap_or(20) as i32, i(a, "height").unwrap_or(14) as i32);
            let scene_id = e.with_world(|w| {
                let mut sc = Scene::new(&name, wd.clamp(8, 40), ht.clamp(8, 30));
                sc.description = desc.clone();
                let id = sc.id.clone();
                w.scenes.push(sc);
                w.active_scene = Some(id.clone());
                w.story_mode = false;
                tool_line(w, format!("GM opened a new scene: {name}"));
                Ok(id)
            })?;
            let mut msg = format!("Scene “{name}” ({scene_id}) is active. Place tokens next.");
            if gen {
                msg.push_str(&request_art(e, ImageKind::Map, s(a, "map_prompt").unwrap_or(&desc), None));
            }
            Ok(msg)
        }
        "activate_scene" => e.with_world(|w| {
            let q = req(a, "scene")?;
            let id = w.scenes.iter().find(|s| s.id == q || s.name.eq_ignore_ascii_case(q)).map(|s| s.id.clone()).ok_or("no such scene")?;
            w.active_scene = Some(id);
            Ok("ok".into())
        }),
        "place_token" | "move_token" => e.with_world(|w| {
            let id = find_id(w, req(a, "actor")?)?;
            let (x, y) = (i(a, "x").ok_or("missing x")? as i32, i(a, "y").ok_or("missing y")? as i32);
            let hidden = b(a, "hidden").unwrap_or(false);
            let is_pc = w.pc().map(|p| p.id == id).unwrap_or(false);
            let nm = w.actor(&id).unwrap().name.clone();
            let sc = w.scene_mut().ok_or("no active scene; create_scene first")?;
            if !sc.in_bounds(x, y) {
                return Err(format!("({x}, {y}) is off the {}×{} map", sc.width, sc.height));
            }
            if let Some(occ) = sc.tokens.iter().find(|t| t.x == x && t.y == y && t.actor_id != id) {
                let _ = occ;
                return Err(format!("({x}, {y}) is occupied; pick an adjacent square"));
            }
            let moved = if let Some(t) = sc.tokens.iter_mut().find(|t| t.actor_id == id) {
                let ft = grid_distance_ft(t.x, t.y, x, y);
                t.x = x;
                t.y = y;
                if name == "place_token" { t.hidden = hidden; }
                Some(ft)
            } else {
                if name == "move_token" {
                    return Err(format!("{nm} has no token here; use place_token"));
                }
                sc.tokens.push(Token { id: new_id(), actor_id: id.clone(), x, y, hidden });
                None
            };
            if is_pc {
                sc.reveal_radius(x, y, 6);
            }
            if let Some(ft) = moved {
                if !hidden {
                    tool_line(w, format!("GM moved {nm} {ft} ft to ({x}, {y})"));
                }
            }
            Ok(format!("{nm} at ({x}, {y})"))
        }),
        "remove_token" => e.with_world(|w| {
            let id = find_id(w, req(a, "actor")?)?;
            let sc = w.scene_mut().ok_or("no active scene")?;
            sc.tokens.retain(|t| t.actor_id != id);
            Ok("removed".into())
        }),
        "reveal_fog" => e.with_world(|w| {
            let sc = w.scene_mut().ok_or("no active scene")?;
            let n = if b(a, "all").unwrap_or(false) {
                let (wd, ht) = (sc.width, sc.height);
                sc.reveal_rect(0, 0, wd, ht)
            } else {
                sc.reveal_rect(i(a, "x").unwrap_or(0) as i32, i(a, "y").unwrap_or(0) as i32, i(a, "w").unwrap_or(1) as i32, i(a, "h").unwrap_or(1) as i32)
            };
            tool_line(w, format!("GM revealed {n} cells"));
            Ok(format!("revealed {n} cells"))
        }),
        "start_combat" => e.with_world(|w| {
            let mut ids = vec![];
            for q in strs(a, "actors") {
                ids.push(find_id(w, &q)?);
            }
            if let Some(pc) = w.pc() {
                if !ids.contains(&pc.id) {
                    ids.insert(0, pc.id.clone());
                }
            }
            let hidden: Vec<String> = strs(a, "hidden").iter().filter_map(|q| w.actor(q).map(|x| x.id.clone())).collect();
            let mut c = e.with_rng(|rng| combat::roll_initiative(w, &ids, rng));
            c.name = s(a, "name").unwrap_or("Combat").into();
            for cb in &mut c.order {
                cb.hidden = hidden.contains(&cb.actor_id);
            }
            let order = c.order.iter().map(|cb| format!("{} {}", w.actor(&cb.actor_id).map(|x| x.name.as_str()).unwrap_or("?"), cb.initiative)).collect::<Vec<_>>().join(", ");
            w.combat = Some(c);
            w.story_mode = false;
            tool_line(w, format!("Initiative: {order}"));
            let first = combat::current_actor(w).map(|x| x.name.clone()).unwrap_or_default();
            Ok(format!("Round 1. Order: {order}. First: {first}"))
        }),
        "next_turn" => e.with_world(|w| {
            let id = combat::next_turn(w).ok_or("no combat running (or everyone is down)")?;
            let c = w.combat.as_ref().unwrap();
            let nm = w.actor(&id).map(|x| x.name.clone()).unwrap_or_default();
            let is_pc = w.pc().map(|p| p.id == id).unwrap_or(false);
            Ok(format!("Round {}: {nm}'s turn{}", c.round, if is_pc { " — the player's turn: stop and hand over" } else { "" }))
        }),
        "end_combat" => e.with_world(|w| {
            w.combat = None;
            tool_line(w, s(a, "summary").map(|x| format!("Combat ended: {x}")).unwrap_or_else(|| "Combat ended".into()));
            Ok("combat ended".into())
        }),
        "journal_write" => e.with_world(|w| {
            let active = b(a, "active").unwrap_or(false);
            if active {
                for j in &mut w.journal {
                    j.active = false;
                }
            }
            let entry = JournalEntry {
                id: new_id(),
                title: req(a, "title")?.into(),
                category: s(a, "category").unwrap_or("lore").to_lowercase(),
                body: req(a, "body")?.into(),
                image: None,
                objectives: strs(a, "objectives").into_iter().map(|t| Objective { text: t, done: false }).collect(),
                active,
                updated_at: now_ms(),
                by_player: false,
            };
            tool_line(w, format!("GM wrote in the journal: {}", entry.title));
            w.journal.push(entry);
            Ok("written".into())
        }),
        "journal_update" => e.with_world(|w| {
            let q = req(a, "title")?;
            let j = w.journal.iter_mut().find(|j| j.id == q || j.title.eq_ignore_ascii_case(q)).ok_or("no such journal entry")?;
            if let Some(body) = s(a, "body") {
                j.body = body.into();
            }
            for o in strs(a, "add_objectives") {
                j.objectives.push(Objective { text: o, done: false });
            }
            let done = strs(a, "complete");
            for o in &mut j.objectives {
                if done.iter().any(|d| o.text.to_lowercase().contains(&d.to_lowercase())) {
                    o.done = true;
                }
            }
            if let Some(act) = b(a, "active") {
                j.active = act;
            }
            j.updated_at = now_ms();
            let t = j.title.clone();
            tool_line(w, format!("GM updated the journal: {t}"));
            Ok("updated".into())
        }),
        "generate_image" => {
            let kind = match req(a, "kind")? {
                "map" => ImageKind::Map,
                "token" | "portrait" => ImageKind::Token,
                _ => ImageKind::Art,
            };
            let actor = match s(a, "actor") {
                Some(q) => Some(e.world().and_then(|w| w.actor(q).map(|x| x.id.clone())).ok_or("no such character")?),
                None => None,
            };
            Ok(request_art(e, kind, req(a, "prompt")?, actor).trim().to_string())
        }
        "story_mode" => e.with_world(|w| {
            w.story_mode = b(a, "on").unwrap_or(true);
            w.story_choices = strs(a, "choices");
            Ok(format!("Story Mode {}", if w.story_mode { "on" } else { "off" }))
        }),
        "suggest_actions" => e.with_world(|w| {
            w.story_choices = strs(a, "actions").into_iter().take(4).collect();
            Ok("offered".into())
        }),
        other => Err(format!("unknown tool {other}")),
    }
}

fn request_art(e: &Arc<Engine>, kind: ImageKind, prompt: &str, actor: Option<String>) -> String {
    let st = e.settings().image;
    if !st.enabled() {
        return " Image generation is disabled (no image endpoint set up); carry on without it.".into();
    }
    if !st.gm_can_generate {
        return " The player turned off GM image generation; carry on without it.".into();
    }
    match art::spawn(e.clone(), ImageRequest { kind, prompt: prompt.to_string(), actor_id: actor }) {
        Ok(()) => " Image requested; it appears on the table when ready (about 10–30 s).".into(),
        Err(x) => format!(" Image not requested: {x}"),
    }
}

pub fn rules_section(topic: &str) -> String {
    let t = topic.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for sec in RULES.split("\n## ").skip(1) {
        let head = sec.lines().next().unwrap_or("").to_lowercase();
        let score = t.split_whitespace().filter(|w| head.contains(w) || sec.to_lowercase().contains(w)).count() * 2 + usize::from(head.contains(&t)) * 5;
        if score > 0 && best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, format!("## {sec}")));
        }
    }
    best.map(|(_, s)| s.trim().to_string()).unwrap_or_else(|| {
        let heads: Vec<&str> = RULES.split("\n## ").skip(1).filter_map(|s| s.lines().next()).collect();
        format!("No section matched. Topics: {}", heads.join(", "))
    })
}

pub fn state_json(w: &World, images: bool) -> Value {
    let pc = w.pc();
    let scene = w.scene().map(|sc| {
        let tokens: Vec<Value> = sc
            .tokens
            .iter()
            .filter_map(|t| {
                let a = w.actor(&t.actor_id)?;
                Some(json!({"actor": a.name, "id": a.id, "x": t.x, "y": t.y, "hidden": t.hidden, "disposition": a.disposition, "hp": a.hp, "max_hp": a.max_hp, "ac": a.ac, "down": a.is_down(), "minion": a.minion}))
            })
            .collect();
        json!({"name": sc.name, "description": sc.description, "width": sc.width, "height": sc.height, "map": sc.map_status, "tokens": tokens, "revealed_cells": sc.revealed.iter().filter(|r| **r).count()})
    });
    let combat = w.combat.as_ref().map(|c| {
        json!({"round": c.round, "current": combat::current_actor(w).map(|a| a.name.clone()), "order": c.order.iter().map(|cb| json!({"actor": w.actor(&cb.actor_id).map(|a| a.name.clone()), "initiative": cb.initiative, "hidden": cb.hidden})).collect::<Vec<_>>()})
    });
    let hero = pc.map(|p| {
        json!({
            "name": p.name, "id": p.id, "level": p.level, "class": p.class, "ancestry": p.ancestry,
            "hp": p.hp, "max_hp": p.max_hp, "temp_hp": p.temp_hp, "ac": p.ac, "speed": p.speed, "fate": p.fate, "xp": p.xp,
            "abilities": p.abilities, "attacks": p.attacks.iter().map(|a| format!("{} {} {} {}", a.name, signed(a.to_hit), a.damage, a.damage_type)).collect::<Vec<_>>(),
            "spells": p.spells.iter().map(|s| s.name.clone()).collect::<Vec<_>>(), "slots": p.spell_slots,
            "conditions": p.conditions, "inventory": p.inventory.iter().map(|i| format!("{} ×{}", i.name, i.qty)).collect::<Vec<_>>(),
            "appearance": p.appearance, "background": p.background
        })
    });
    let others: Vec<Value> = w.actors.iter().filter(|a| a.kind != ActorKind::Pc).map(|a| json!({"name": a.name, "kind": a.kind, "disposition": a.disposition, "hp": a.hp, "max_hp": a.max_hp, "down": a.is_down()})).collect();
    json!({
        "world": {"name": w.name, "pitch": w.pitch, "tone": w.tone, "rules_dial": w.rules_dial, "session": w.session, "tension": w.tension, "story_mode": w.story_mode},
        "hero": hero,
        "characters": others,
        "scene": scene,
        "scenes": w.scenes.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
        "combat": combat,
        "journal": w.journal.iter().map(|j| json!({"title": j.title, "category": j.category, "active": j.active, "objectives": j.objectives})).collect::<Vec<_>>(),
        "pending_sheet_proposals": w.proposals.len(),
        "events_since_last_turn": w.pending_events,
        "image_generation": images,
    })
}
