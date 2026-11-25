//! Character creation: point buy, ancestry, class presets tuned for one hero.

use serde::{Deserialize, Serialize};

use super::{modifier, Ability, Skill};
use crate::world::{new_id, Abilities, Actor, ActorKind, Attack, Disposition, Item, Spell};

pub const POINT_BUY_BUDGET: i32 = 27;

pub fn point_cost(score: i32) -> Option<i32> {
    Some(match score {
        8 => 0,
        9 => 1,
        10 => 2,
        11 => 3,
        12 => 4,
        13 => 5,
        14 => 7,
        15 => 9,
        _ => return None,
    })
}

pub fn point_buy_spent(a: &Abilities) -> Option<i32> {
    Ability::ALL.iter().map(|x| point_cost(a.score(*x))).sum()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeroSpec {
    pub name: String,
    pub ancestry: String,
    pub class: String,
    /// Base scores before ancestry bonuses (point buy).
    pub scores: Abilities,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub background: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassInfo {
    pub name: &'static str,
    pub hit_die: i32,
    pub blurb: &'static str,
    pub solo: &'static str,
}

pub const CLASSES: [ClassInfo; 6] = [
    ClassInfo { name: "Fighter", hit_die: 10, blurb: "Armour, weapons, Second Wind.", solo: "Second Wind recharges on a short rest and also ends 1 condition." },
    ClassInfo { name: "Rogue", hit_die: 8, blurb: "Sneak Attack, skills, Cunning Action.", solo: "Sneak Attack works whenever the scene distracts your target." },
    ClassInfo { name: "Cleric", hit_die: 8, blurb: "Divine magic, healing, armour.", solo: "Healing Word on yourself for free, once per short rest." },
    ClassInfo { name: "Wizard", hit_die: 6, blurb: "Wide spellbook, rituals.", solo: "Arcane Ward from level 1: temp HP equal to 2 × level + INT." },
    ClassInfo { name: "Ranger", hit_die: 10, blurb: "Bows, tracking, Hunter’s Mark.", solo: "Favored Foe: +1d4 once per turn against your marked target." },
    ClassInfo { name: "Paladin", hit_die: 10, blurb: "Heavy armour, smites, auras.", solo: "Lay on Hands pool doubles and costs a bonus action." },
];

pub fn ancestry_bonus(ancestry: &str) -> Abilities {
    let z = Abilities::default();
    match ancestry.to_lowercase().as_str() {
        "human" => Abilities { str: 1, dex: 1, con: 1, int: 1, wis: 1, cha: 1 },
        "wood elf" | "elf" => Abilities { dex: 2, wis: 1, ..z },
        "dwarf" => Abilities { con: 2, wis: 1, ..z },
        "halfling" => Abilities { dex: 2, cha: 1, ..z },
        "half-orc" | "half orc" => Abilities { str: 2, con: 1, ..z },
        _ => Abilities { dex: 1, con: 1, wis: 1, ..z },
    }
}

fn add(a: Abilities, b: Abilities) -> Abilities {
    Abilities { str: a.str + b.str, dex: a.dex + b.dex, con: a.con + b.con, int: a.int + b.int, wis: a.wis + b.wis, cha: a.cha + b.cha }
}

pub fn validate(spec: &HeroSpec) -> Result<(), String> {
    if spec.name.trim().is_empty() {
        return Err("Give your hero a name.".into());
    }
    match point_buy_spent(&spec.scores) {
        None => Err("Each score must be 8–15 before ancestry bonuses.".into()),
        Some(n) if n > POINT_BUY_BUDGET => Err(format!("That spends {n} points; the budget is {POINT_BUY_BUDGET}.")),
        _ => Ok(()),
    }
}

/// Build a level-1 hero with the solo bonuses (HP + CON×2, 3 Fate).
#[allow(clippy::type_complexity)]
pub fn build_hero(spec: &HeroSpec) -> Result<Actor, String> {
    validate(spec)?;
    let ab = add(spec.scores, ancestry_bonus(&spec.ancestry));
    let m = |a: Ability| modifier(ab.score(a));
    let (str_m, dex_m, con_m, wis_m, int_m) = (m(Ability::Str), m(Ability::Dex), m(Ability::Con), m(Ability::Wis), m(Ability::Int));
    let class = CLASSES.iter().find(|c| c.name.eq_ignore_ascii_case(&spec.class)).unwrap_or(&CLASSES[0]);
    let pb = 2;
    let melee = |name: &str, die: &str, ab_mod: i32| Attack {
        name: name.into(),
        to_hit: ab_mod + pb,
        damage: format!("{die}{}{}", if ab_mod >= 0 { "+" } else { "" }, ab_mod),
        damage_type: String::new(),
        range: "melee".into(),
    };
    use Skill::*;
    let (saves, skills, ac, attacks, spells, slots, items): (Vec<Ability>, Vec<Skill>, i32, Vec<Attack>, Vec<Spell>, Vec<u8>, Vec<&str>) =
        match class.name {
            "Fighter" => (
                vec![Ability::Str, Ability::Con],
                vec![Athletics, Perception],
                16,
                vec![melee("Longsword", "1d8", str_m), Attack { range: "20/60 ft".into(), ..melee("Handaxe", "1d6", str_m) }],
                vec![],
                vec![],
                vec!["Chain mail (worn)", "Longsword", "Handaxe", "Explorer’s pack"],
            ),
            "Rogue" => (
                vec![Ability::Dex, Ability::Int],
                vec![Stealth, Acrobatics, SleightOfHand, Perception],
                11 + dex_m,
                vec![melee("Shortsword", "1d6", dex_m), Attack { range: "80/320 ft".into(), ..melee("Shortbow", "1d6", dex_m) }],
                vec![],
                vec![],
                vec!["Leather armour (worn)", "Shortsword", "Shortbow", "Thieves’ tools"],
            ),
            "Cleric" => (
                vec![Ability::Wis, Ability::Cha],
                vec![Medicine, Religion],
                16 + dex_m.min(2),
                vec![melee("Mace", "1d6", str_m), Attack { name: "Sacred Flame".into(), to_hit: 0, damage: "1d8".into(), damage_type: "radiant (DEX save)".into(), range: "60 ft".into() }],
                vec![
                    Spell { name: "Cure Wounds".into(), level: 1, summary: format!("Action · touch · heal 1d8{}", super::signed(wis_m)) },
                    Spell { name: "Bless".into(), level: 1, summary: "Concentration · +1d4 to attacks and saves".into() },
                    Spell { name: "Guiding Bolt".into(), level: 1, summary: "Ranged spell attack · 4d6 radiant".into() },
                ],
                vec![2],
                vec!["Scale mail (worn)", "Shield", "Mace", "Holy symbol"],
            ),
            "Wizard" => (
                vec![Ability::Int, Ability::Wis],
                vec![Arcana, History],
                10 + dex_m,
                vec![melee("Quarterstaff", "1d6", str_m), Attack { name: "Fire Bolt".into(), to_hit: int_m + pb, damage: "1d10".into(), damage_type: "fire".into(), range: "120 ft".into() }],
                vec![
                    Spell { name: "Magic Missile".into(), level: 1, summary: "3 darts · 1d4+1 force each, never miss".into() },
                    Spell { name: "Shield".into(), level: 1, summary: "Reaction · +5 AC until your next turn".into() },
                    Spell { name: "Sleep".into(), level: 1, summary: "5d8 HP of creatures fall asleep".into() },
                ],
                vec![2],
                vec!["Spellbook", "Quarterstaff", "Arcane focus", "Scholar’s pack"],
            ),
            "Paladin" => (
                vec![Ability::Wis, Ability::Cha],
                vec![Athletics, Persuasion],
                18,
                vec![melee("Longsword", "1d8", str_m), Attack { range: "20/60 ft".into(), ..melee("Javelin", "1d6", str_m) }],
                vec![Spell { name: "Lay on Hands".into(), level: 0, summary: "Bonus action · heal from a pool of 10 HP".into() }],
                vec![],
                vec!["Chain mail (worn)", "Shield", "Longsword", "Javelins ×5"],
            ),
            _ => (
                vec![Ability::Str, Ability::Dex],
                vec![Stealth, Perception, Survival],
                12 + dex_m,
                vec![Attack { to_hit: dex_m + pb + 2, range: "150/600 ft".into(), damage_type: "piercing".into(), ..melee("Longbow", "1d8", dex_m) }, melee("Shortsword", "1d6", dex_m.max(str_m))],
                vec![Spell { name: "Hunter’s Mark".into(), level: 1, summary: "Bonus action · 90 ft · +1d6 to the marked target".into() }],
                vec![],
                vec!["Studded leather (worn)", "Longbow", "Arrows ×20", "Shortsword"],
            ),
        };
    let max_hp = (class.hit_die + con_m + (con_m * 2).max(0)).max(1);
    let inventory = items
        .into_iter()
        .map(|s| {
            let (n, q) = match s.rsplit_once(" ×") {
                Some((n, q)) => (n.to_string(), q.parse().unwrap_or(1)),
                None => (s.to_string(), 1),
            };
            Item { name: n, qty: q, note: String::new() }
        })
        .chain([Item { name: "Healing potion".into(), qty: 2, note: "2d4+2".into() }])
        .collect();
    let _ = wis_m;
    Ok(Actor {
        id: new_id(),
        name: spec.name.trim().to_string(),
        kind: ActorKind::Pc,
        disposition: Disposition::Friendly,
        level: 1,
        class: class.name.into(),
        ancestry: spec.ancestry.clone(),
        background: spec.background.clone(),
        abilities: ab,
        save_profs: saves,
        skill_profs: skills,
        max_hp,
        hp: max_hp,
        temp_hp: 0,
        ac,
        speed: if spec.ancestry.to_lowercase().contains("elf") { 35 } else if spec.ancestry.eq_ignore_ascii_case("halfling") || spec.ancestry.eq_ignore_ascii_case("dwarf") { 25 } else { 30 },
        attacks,
        spell_slots_max: slots.clone(),
        spell_slots: slots,
        spells,
        concentrating: None,
        inventory,
        gold: 15,
        conditions: vec![],
        fate: 3,
        xp: 0,
        hit_dice: 1,
        portrait: None,
        appearance: spec.appearance.clone(),
        minion: false,
        cr: String::new(),
        traits: vec![class.solo.to_string()],
        notes: String::new(),
    })
}

/// Aria Thorne, the hero from the design mockups (level 3 ranger).
pub fn sample_hero() -> Actor {
    let mut a = build_hero(&HeroSpec {
        name: "Aria Thorne".into(),
        ancestry: "Wood Elf".into(),
        class: "Ranger".into(),
        scores: Abilities { str: 10, dex: 15, con: 14, int: 10, wis: 13, cha: 8 },
        appearance: "Lean wood elf in a moss-green hooded cloak, braided copper hair, a scar across the left cheek.".into(),
        background: "Raised by the wardens who failed to stop the fire.".into(),
    })
    .expect("sample hero is valid");
    a.level = 3;
    a.max_hp = 31;
    a.hp = 24;
    a.xp = 1250;
    a.fate = 2;
    a.hit_dice = 3;
    a.spell_slots_max = vec![3];
    a.spell_slots = vec![2];
    a.skill_profs.extend([Skill::Athletics, Skill::Nature, Skill::AnimalHandling]);
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_buy_budget() {
        let ok = Abilities { str: 10, dex: 15, con: 14, int: 10, wis: 13, cha: 8 };
        assert_eq!(point_buy_spent(&ok), Some(25));
        let over = Abilities { str: 15, dex: 15, con: 15, int: 8, wis: 8, cha: 8 };
        assert_eq!(point_buy_spent(&over), Some(27));
        let bad = Abilities { str: 16, ..ok };
        assert_eq!(point_buy_spent(&bad), None);
    }

    #[test]
    fn hero_gets_solo_bonuses() {
        let spec = HeroSpec {
            name: "Cass".into(),
            ancestry: "Human".into(),
            class: "Fighter".into(),
            scores: Abilities { str: 15, dex: 13, con: 14, int: 8, wis: 12, cha: 10 },
            appearance: String::new(),
            background: String::new(),
        };
        let h = build_hero(&spec).unwrap();
        // CON 14+1=15 → +2; HP = 10 + 2 + 4 (solo)
        assert_eq!(h.max_hp, 16);
        assert_eq!(h.fate, 3);
        assert_eq!(h.attacks[0].to_hit, 3 + 2);
        assert_eq!(h.ac, 16);
    }

    #[test]
    fn rejects_empty_name_and_overspend() {
        let mut spec = HeroSpec { name: " ".into(), ancestry: "Human".into(), class: "Rogue".into(), scores: Abilities { str: 8, dex: 8, con: 8, int: 8, wis: 8, cha: 8 }, appearance: String::new(), background: String::new() };
        assert!(validate(&spec).is_err());
        spec.name = "Vex".into();
        assert!(validate(&spec).is_ok());
        spec.scores = Abilities { str: 15, dex: 15, con: 15, int: 15, wis: 8, cha: 8 };
        assert!(validate(&spec).unwrap_err().contains("budget"));
    }

    #[test]
    fn sample_hero_matches_design() {
        let a = sample_hero();
        assert_eq!(a.abilities.dex, 17);
        assert_eq!(a.ac, 15);
        assert_eq!(a.attacks[0].to_hit, 7);
        assert_eq!(a.attacks[0].damage, "1d8+3");
        assert_eq!(a.speed, 35);
    }
}
