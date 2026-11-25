//! Built-in compendium (adapted from the SRD 5.1, CC-BY-4.0).

use serde::{Deserialize, Serialize};

use crate::world::{new_id, Abilities, Actor, ActorKind, Attack, Disposition};

const MONSTERS: &str = include_str!("../../../../assets/compendium/monsters.json");
const ITEMS: &str = include_str!("../../../../assets/compendium/items.json");
const SPELLS: &str = include_str!("../../../../assets/compendium/spells.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonsterEntry {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub cr: String,
    pub ac: i32,
    pub hp: i32,
    pub hp_dice: String,
    pub speed: i32,
    pub abilities: [i32; 6],
    #[serde(default)]
    pub minion: bool,
    pub attacks: Vec<Attack>,
    #[serde(default)]
    pub traits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleEntry {
    pub id: String,
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub level: Option<u8>,
    #[serde(default)]
    pub school: String,
    pub summary: String,
    #[serde(default)]
    pub cost: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Compendium {
    pub monsters: Vec<MonsterEntry>,
    pub items: Vec<SimpleEntry>,
    pub spells: Vec<SimpleEntry>,
}

pub fn all() -> Compendium {
    Compendium {
        monsters: serde_json::from_str(MONSTERS).expect("monsters.json"),
        items: serde_json::from_str(ITEMS).expect("items.json"),
        spells: serde_json::from_str(SPELLS).expect("spells.json"),
    }
}

pub fn monster(id_or_name: &str) -> Option<Actor> {
    let q = id_or_name.trim().to_lowercase();
    let c = all();
    let m = c.monsters.iter().find(|m| m.id == q || m.name.to_lowercase() == q)?;
    Some(from_entry(m))
}

pub fn from_entry(m: &MonsterEntry) -> Actor {
    let [s, d, co, i, w, ch] = m.abilities;
    Actor {
        id: new_id(),
        name: m.name.clone(),
        kind: ActorKind::Monster,
        disposition: Disposition::Hostile,
        level: 1,
        class: m.kind.clone(),
        ancestry: String::new(),
        background: String::new(),
        abilities: Abilities { str: s, dex: d, con: co, int: i, wis: w, cha: ch },
        save_profs: vec![],
        skill_profs: vec![],
        max_hp: m.hp,
        hp: m.hp,
        temp_hp: 0,
        ac: m.ac,
        speed: m.speed,
        attacks: m.attacks.clone(),
        spells: vec![],
        spell_slots: vec![],
        spell_slots_max: vec![],
        concentrating: None,
        inventory: vec![],
        gold: 0,
        conditions: vec![],
        fate: 0,
        xp: 0,
        hit_dice: 0,
        portrait: None,
        appearance: String::new(),
        minion: m.minion,
        cr: m.cr.clone(),
        traits: m.traits.clone(),
        notes: String::new(),
    }
}

/// Case-insensitive substring search across all three lists.
pub fn search(query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    let c = all();
    let mut out = vec![];
    for m in &c.monsters {
        if m.name.to_lowercase().contains(&q) || m.kind.to_lowercase().contains(&q) {
            out.push(format!("monster {} — {} {}, CR {}, AC {}, HP {} ({}), {}", m.id, m.name, m.kind, m.cr, m.ac, m.hp, m.hp_dice, if m.minion { "minion" } else { "standard" }));
        }
    }
    for it in &c.items {
        if it.name.to_lowercase().contains(&q) {
            out.push(format!("item {} — {}: {}", it.id, it.name, it.summary));
        }
    }
    for s in &c.spells {
        if s.name.to_lowercase().contains(&q) {
            out.push(format!("spell {} — {} (level {}): {}", s.id, s.name, s.level.unwrap_or(0), s.summary));
        }
    }
    out
}

/// XP awarded for defeating a monster of this CR (SRD), halved for minions.
pub fn xp_for_cr(cr: &str) -> u32 {
    match cr {
        "0" => 10,
        "1/8" => 25,
        "1/4" => 50,
        "1/2" => 100,
        "1" => 200,
        "2" => 450,
        "3" => 700,
        "4" => 1100,
        "5" => 1800,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compendium_loads_and_searches() {
        let c = all();
        assert!(c.monsters.len() >= 15 && c.items.len() >= 15 && c.spells.len() >= 15);
        assert!(search("gob").len() >= 2);
        let g = monster("Goblin").unwrap();
        assert_eq!((g.ac, g.hp, g.minion), (15, 7, true));
        assert_eq!(xp_for_cr("1/4"), 50);
    }
}
