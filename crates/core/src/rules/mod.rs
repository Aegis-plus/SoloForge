//! Solo 5e-inspired rules. Core mechanics follow the SRD 5.1 (CC-BY-4.0);
//! solo adjustments (Fate, minions, Tension) are SoloForge's own.

pub mod checks;
pub mod chargen;
pub mod combat;
pub mod compendium;
pub mod dice;
pub mod oracle;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ability {
    Str,
    Dex,
    Con,
    Int,
    Wis,
    Cha,
}

impl Ability {
    pub const ALL: [Ability; 6] = [Ability::Str, Ability::Dex, Ability::Con, Ability::Int, Ability::Wis, Ability::Cha];
    pub fn short(self) -> &'static str {
        match self {
            Ability::Str => "STR",
            Ability::Dex => "DEX",
            Ability::Con => "CON",
            Ability::Int => "INT",
            Ability::Wis => "WIS",
            Ability::Cha => "CHA",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Ability::Str => "Strength",
            Ability::Dex => "Dexterity",
            Ability::Con => "Constitution",
            Ability::Int => "Intelligence",
            Ability::Wis => "Wisdom",
            Ability::Cha => "Charisma",
        }
    }
    pub fn parse(s: &str) -> Option<Ability> {
        let s = s.trim().to_lowercase();
        Ability::ALL.into_iter().find(|a| a.short().eq_ignore_ascii_case(&s) || a.name().eq_ignore_ascii_case(&s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Acrobatics,
    AnimalHandling,
    Arcana,
    Athletics,
    Deception,
    History,
    Insight,
    Intimidation,
    Investigation,
    Medicine,
    Nature,
    Perception,
    Performance,
    Persuasion,
    Religion,
    SleightOfHand,
    Stealth,
    Survival,
}

impl Skill {
    pub const ALL: [Skill; 18] = [
        Skill::Acrobatics,
        Skill::AnimalHandling,
        Skill::Arcana,
        Skill::Athletics,
        Skill::Deception,
        Skill::History,
        Skill::Insight,
        Skill::Intimidation,
        Skill::Investigation,
        Skill::Medicine,
        Skill::Nature,
        Skill::Perception,
        Skill::Performance,
        Skill::Persuasion,
        Skill::Religion,
        Skill::SleightOfHand,
        Skill::Stealth,
        Skill::Survival,
    ];
    pub fn ability(self) -> Ability {
        use Skill::*;
        match self {
            Athletics => Ability::Str,
            Acrobatics | SleightOfHand | Stealth => Ability::Dex,
            Arcana | History | Investigation | Nature | Religion => Ability::Int,
            AnimalHandling | Insight | Medicine | Perception | Survival => Ability::Wis,
            Deception | Intimidation | Performance | Persuasion => Ability::Cha,
        }
    }
    pub fn name(self) -> &'static str {
        use Skill::*;
        match self {
            Acrobatics => "Acrobatics",
            AnimalHandling => "Animal Handling",
            Arcana => "Arcana",
            Athletics => "Athletics",
            Deception => "Deception",
            History => "History",
            Insight => "Insight",
            Intimidation => "Intimidation",
            Investigation => "Investigation",
            Medicine => "Medicine",
            Nature => "Nature",
            Perception => "Perception",
            Performance => "Performance",
            Persuasion => "Persuasion",
            Religion => "Religion",
            SleightOfHand => "Sleight of Hand",
            Stealth => "Stealth",
            Survival => "Survival",
        }
    }
    pub fn parse(s: &str) -> Option<Skill> {
        let norm = |x: &str| x.to_lowercase().replace([' ', '_', '-'], "");
        let q = norm(s);
        Skill::ALL.into_iter().find(|k| norm(k.name()) == q)
    }
}

pub fn modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

pub fn prof_bonus(level: u8) -> i32 {
    2 + ((level.clamp(1, 20) as i32 - 1) / 4)
}

/// "+5", "−1" (true minus sign).
pub fn signed(n: i32) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        format!("−{}", -n)
    }
}

/// XP needed for each level 1–10 (SRD table).
pub const XP_TABLE: [u32; 10] = [0, 300, 900, 2700, 6500, 14000, 23000, 34000, 48000, 64000];

pub fn xp_for_next(level: u8) -> Option<u32> {
    XP_TABLE.get(level as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modifiers_and_proficiency() {
        assert_eq!(modifier(10), 0);
        assert_eq!(modifier(8), -1);
        assert_eq!(modifier(9), -1);
        assert_eq!(modifier(17), 3);
        assert_eq!(prof_bonus(1), 2);
        assert_eq!(prof_bonus(4), 2);
        assert_eq!(prof_bonus(5), 3);
        assert_eq!(prof_bonus(9), 4);
        assert_eq!(signed(-1), "−1");
    }
    #[test]
    fn parses_names() {
        assert_eq!(Skill::parse("sleight of hand"), Some(Skill::SleightOfHand));
        assert_eq!(Skill::parse("Animal_Handling"), Some(Skill::AnimalHandling));
        assert_eq!(Ability::parse("dex"), Some(Ability::Dex));
        assert_eq!(Ability::parse("Wisdom"), Some(Ability::Wis));
        assert_eq!(xp_for_next(3), Some(2700));
    }
}
