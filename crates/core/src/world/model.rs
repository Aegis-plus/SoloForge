//! The world: everything on the table, serialised to `world.json`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::rules::{Ability, Skill};

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    Uuid::new_v4().simple().to_string()[..12].to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct World {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub pitch: String,
    #[serde(default)]
    pub tone: Vec<String>,
    #[serde(default = "default_dial")]
    pub rules_dial: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub session: u32,
    /// Claude Code session id, so the GM keeps its memory across launches.
    #[serde(default)]
    pub gm_session_id: Option<String>,
    #[serde(default)]
    pub actors: Vec<Actor>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub active_scene: Option<String>,
    #[serde(default)]
    pub combat: Option<Combat>,
    #[serde(default)]
    pub journal: Vec<JournalEntry>,
    #[serde(default)]
    pub chat: Vec<ChatEntry>,
    /// 1–9; drives oracle random events.
    #[serde(default = "default_tension")]
    pub tension: u8,
    /// Table events since the GM's last turn ("Player moved Aria to (5, 7)").
    #[serde(default)]
    pub pending_events: Vec<String>,
    #[serde(default)]
    pub proposals: Vec<SheetProposal>,
    /// Story Mode (map hidden) vs Map mode.
    #[serde(default)]
    pub story_mode: bool,
    #[serde(default)]
    pub story_choices: Vec<String>,
}

fn default_tension() -> u8 {
    5
}
fn default_dial() -> String {
    "balanced".into()
}

impl World {
    pub fn new(name: &str, pitch: &str) -> Self {
        let t = now_ms();
        World {
            id: new_id(),
            name: name.to_string(),
            pitch: pitch.to_string(),
            tone: vec![],
            rules_dial: default_dial(),
            created_at: t,
            updated_at: t,
            session: 1,
            gm_session_id: None,
            actors: vec![],
            scenes: vec![],
            active_scene: None,
            combat: None,
            journal: vec![],
            chat: vec![],
            tension: default_tension(),
            pending_events: vec![],
            proposals: vec![],
            story_mode: false,
            story_choices: vec![],
        }
    }

    pub fn pc(&self) -> Option<&Actor> {
        self.actors.iter().find(|a| a.kind == ActorKind::Pc)
    }
    pub fn pc_mut(&mut self) -> Option<&mut Actor> {
        self.actors.iter_mut().find(|a| a.kind == ActorKind::Pc)
    }
    pub fn actor(&self, id_or_name: &str) -> Option<&Actor> {
        let q = id_or_name.trim();
        self.actors
            .iter()
            .find(|a| a.id == q)
            .or_else(|| self.actors.iter().find(|a| a.name.eq_ignore_ascii_case(q)))
    }
    pub fn actor_mut(&mut self, id_or_name: &str) -> Option<&mut Actor> {
        let q = id_or_name.trim().to_string();
        let idx = self
            .actors
            .iter()
            .position(|a| a.id == q)
            .or_else(|| self.actors.iter().position(|a| a.name.eq_ignore_ascii_case(&q)))?;
        self.actors.get_mut(idx)
    }
    pub fn scene(&self) -> Option<&Scene> {
        let id = self.active_scene.as_ref()?;
        self.scenes.iter().find(|s| &s.id == id)
    }
    pub fn scene_mut(&mut self) -> Option<&mut Scene> {
        let id = self.active_scene.clone()?;
        self.scenes.iter_mut().find(|s| s.id == id)
    }
    pub fn push_chat(&mut self, kind: ChatKind) -> ChatEntry {
        let e = ChatEntry { id: new_id(), ts: now_ms(), kind };
        self.chat.push(e.clone());
        e
    }
    pub fn note_event(&mut self, s: impl Into<String>) {
        self.pending_events.push(s.into());
        if self.pending_events.len() > 40 {
            let drop = self.pending_events.len() - 40;
            self.pending_events.drain(0..drop);
        }
    }
    pub fn touch(&mut self) {
        self.updated_at = now_ms();
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Pc,
    Companion,
    Npc,
    Monster,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Friendly,
    Neutral,
    Hostile,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Abilities {
    pub str: i32,
    pub dex: i32,
    pub con: i32,
    pub int: i32,
    pub wis: i32,
    pub cha: i32,
}

impl Abilities {
    pub fn score(&self, a: Ability) -> i32 {
        match a {
            Ability::Str => self.str,
            Ability::Dex => self.dex,
            Ability::Con => self.con,
            Ability::Int => self.int,
            Ability::Wis => self.wis,
            Ability::Cha => self.cha,
        }
    }
    pub fn modifier(&self, a: Ability) -> i32 {
        crate::rules::modifier(self.score(a))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attack {
    pub name: String,
    pub to_hit: i32,
    /// Dice expression, e.g. "1d8+3".
    pub damage: String,
    #[serde(default)]
    pub damage_type: String,
    #[serde(default)]
    pub range: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub name: String,
    #[serde(default = "one")]
    pub qty: u32,
    #[serde(default)]
    pub note: String,
}
fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Spell {
    pub name: String,
    pub level: u8,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub kind: ActorKind,
    pub disposition: Disposition,
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub ancestry: String,
    #[serde(default)]
    pub background: String,
    pub abilities: Abilities,
    #[serde(default)]
    pub save_profs: Vec<Ability>,
    #[serde(default)]
    pub skill_profs: Vec<Skill>,
    pub max_hp: i32,
    pub hp: i32,
    #[serde(default)]
    pub temp_hp: i32,
    pub ac: i32,
    #[serde(default = "default_speed")]
    pub speed: i32,
    #[serde(default)]
    pub attacks: Vec<Attack>,
    #[serde(default)]
    pub spells: Vec<Spell>,
    #[serde(default)]
    pub spell_slots: Vec<u8>,
    #[serde(default)]
    pub spell_slots_max: Vec<u8>,
    #[serde(default)]
    pub concentrating: Option<String>,
    #[serde(default)]
    pub inventory: Vec<Item>,
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub fate: u8,
    #[serde(default)]
    pub xp: u32,
    #[serde(default)]
    pub hit_dice: u8,
    #[serde(default)]
    pub portrait: Option<String>,
    #[serde(default)]
    pub appearance: String,
    /// Solo rule: minions drop on any hit.
    #[serde(default)]
    pub minion: bool,
    #[serde(default)]
    pub cr: String,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub notes: String,
}
fn default_speed() -> i32 {
    30
}

impl Actor {
    pub fn prof_bonus(&self) -> i32 {
        crate::rules::prof_bonus(self.level.max(1))
    }
    pub fn is_down(&self) -> bool {
        self.hp <= 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Token {
    pub id: String,
    pub actor_id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub width: i32,
    pub height: i32,
    /// File name under `<world>/images/`.
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub map_status: ImageStatus,
    #[serde(default)]
    pub tokens: Vec<Token>,
    #[serde(default = "yes")]
    pub fog: bool,
    /// Row-major revealed cells (width*height).
    #[serde(default)]
    pub revealed: Vec<bool>,
}
fn yes() -> bool {
    true
}

impl Scene {
    pub fn new(name: &str, width: i32, height: i32) -> Self {
        let width = width.clamp(4, 60);
        let height = height.clamp(4, 60);
        Scene {
            id: new_id(),
            name: name.into(),
            description: String::new(),
            width,
            height,
            background: None,
            map_status: ImageStatus::None,
            tokens: vec![],
            fog: true,
            revealed: vec![false; (width * height) as usize],
        }
    }
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }
    pub fn reveal_rect(&mut self, x: i32, y: i32, w: i32, h: i32) -> usize {
        if self.revealed.len() != (self.width * self.height) as usize {
            self.revealed = vec![false; (self.width * self.height) as usize];
        }
        let mut n = 0;
        for yy in y.max(0)..(y + h).min(self.height) {
            for xx in x.max(0)..(x + w).min(self.width) {
                let i = (yy * self.width + xx) as usize;
                if !self.revealed[i] {
                    self.revealed[i] = true;
                    n += 1;
                }
            }
        }
        n
    }
    /// Reveal a radius (in cells) around a point — the hero's sight.
    pub fn reveal_radius(&mut self, cx: i32, cy: i32, r: i32) -> usize {
        if self.revealed.len() != (self.width * self.height) as usize {
            self.revealed = vec![false; (self.width * self.height) as usize];
        }
        let mut n = 0;
        for yy in (cy - r).max(0)..=(cy + r).min(self.height - 1) {
            for xx in (cx - r).max(0)..=(cx + r).min(self.width - 1) {
                let (dx, dy) = (xx - cx, yy - cy);
                if dx * dx + dy * dy <= r * r {
                    let i = (yy * self.width + xx) as usize;
                    if !self.revealed[i] {
                        self.revealed[i] = true;
                        n += 1;
                    }
                }
            }
        }
        n
    }
    pub fn token_for(&self, actor_id: &str) -> Option<&Token> {
        self.tokens.iter().find(|t| t.actor_id == actor_id)
    }
}

/// 5e grid distance (each diagonal = 5 ft), in feet.
pub fn grid_distance_ft(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs()) * 5
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImageStatus {
    #[default]
    None,
    Pending,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Combatant {
    pub actor_id: String,
    pub initiative: i32,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Combat {
    pub round: u32,
    pub turn: usize,
    pub order: Vec<Combatant>,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JournalEntry {
    pub id: String,
    pub title: String,
    /// quest | npc | lore | handout | note
    pub category: String,
    pub body: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub objectives: Vec<Objective>,
    #[serde(default)]
    pub active: bool,
    pub updated_at: u64,
    #[serde(default)]
    pub by_player: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Objective {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SheetProposal {
    pub id: String,
    pub actor_id: String,
    pub reason: String,
    pub changes: Vec<SheetChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SheetChange {
    AddItem { name: String, qty: u32, #[serde(default)] note: String },
    RemoveItem { name: String, qty: u32 },
    Xp { amount: u32 },
    Gold { amount: i32 },
    Condition { name: String, add: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatEntry {
    pub id: String,
    pub ts: u64,
    #[serde(flatten)]
    pub kind: ChatKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatKind {
    Gm { text: String },
    Player { text: String },
    Roll { card: RollCard },
    Tool { text: String },
    Image { file: Option<String>, prompt: String, status: ImageStatus, #[serde(default)] error: String },
    System { text: String },
    Oracle { question: String, likelihood: String, roll: u32, answer: String, #[serde(default)] event: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollCard {
    pub title: String,
    #[serde(default)]
    pub actor: String,
    pub lines: Vec<RollLine>,
    /// success | fail | crit | fumble | none
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub by_gm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollLine {
    pub label: String,
    pub detail: String,
    pub total: i32,
    /// to_hit | damage | check | plain
    #[serde(default)]
    pub kind: String,
}
