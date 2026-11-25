//! Ability checks, saving throws and skill checks.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::dice;
use super::{signed, Ability, Skill};
use crate::world::{Actor, RollCard, RollLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckKind {
    Ability(Ability),
    Save(Ability),
    Skill(Skill),
}

impl CheckKind {
    /// "stealth", "dex", "dex save", "wisdom saving throw"
    pub fn parse(s: &str) -> Option<CheckKind> {
        let l = s.trim().to_lowercase();
        for suffix in [" saving throw", " save"] {
            if let Some(a) = l.strip_suffix(suffix) {
                return Ability::parse(a).map(CheckKind::Save);
            }
        }
        if let Some(k) = Skill::parse(&l) {
            return Some(CheckKind::Skill(k));
        }
        Ability::parse(&l).map(CheckKind::Ability)
    }
    pub fn label(self) -> String {
        match self {
            CheckKind::Ability(a) => format!("{} Check", a.name()),
            CheckKind::Save(a) => format!("{} Save", a.name()),
            CheckKind::Skill(k) => format!("{} Check", k.name()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Adv {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

impl Adv {
    pub fn parse(s: &str) -> Adv {
        match s.trim().to_lowercase().as_str() {
            "adv" | "advantage" => Adv::Advantage,
            "dis" | "disadvantage" => Adv::Disadvantage,
            _ => Adv::Normal,
        }
    }
    fn suffix(self) -> &'static str {
        match self {
            Adv::Normal => "",
            Adv::Advantage => " adv",
            Adv::Disadvantage => " dis",
        }
    }
}

pub fn bonus(actor: &Actor, kind: CheckKind) -> i32 {
    let pb = actor.prof_bonus();
    match kind {
        CheckKind::Ability(a) => actor.abilities.modifier(a),
        CheckKind::Save(a) => actor.abilities.modifier(a) + if actor.save_profs.contains(&a) { pb } else { 0 },
        CheckKind::Skill(k) => actor.abilities.modifier(k.ability()) + if actor.skill_profs.contains(&k) { pb } else { 0 },
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub total: i32,
    pub natural: u32,
    pub dc: Option<i32>,
    pub success: Option<bool>,
    pub card: RollCard,
}

pub fn check<R: Rng + ?Sized>(actor: &Actor, kind: CheckKind, dc: Option<i32>, adv: Adv, rng: &mut R) -> CheckResult {
    let b = bonus(actor, kind);
    let expr = format!("1d20{}{}", if b >= 0 { "+" } else { "" }, b) + adv.suffix();
    let r = dice::roll(&expr, rng).expect("valid check expression");
    let natural = r.natural_d20().unwrap_or(0);
    let success = dc.map(|dc| r.total >= dc);
    let outcome = match (natural, success) {
        (20, _) => "crit",
        (1, _) => "fumble",
        (_, Some(true)) => "success",
        (_, Some(false)) => "fail",
        _ => "none",
    };
    let mut detail = r.breakdown();
    if let Some(dc) = dc {
        detail.push_str(&format!(" vs DC {dc}"));
    }
    let card = RollCard {
        title: kind.label(),
        actor: actor.name.clone(),
        lines: vec![RollLine { label: kind.label(), detail, total: r.total, kind: "check".into() }],
        outcome: outcome.into(),
        note: String::new(),
        by_gm: false,
    };
    CheckResult { total: r.total, natural, dc, success, card }
}

pub fn describe_bonus(actor: &Actor, kind: CheckKind) -> String {
    signed(bonus(actor, kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::chargen;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn ranger_bonuses_match_the_sheet() {
        let aria = chargen::sample_hero();
        assert_eq!(bonus(&aria, CheckKind::Skill(Skill::Stealth)), 5);
        assert_eq!(bonus(&aria, CheckKind::Skill(Skill::Perception)), 4);
        assert_eq!(bonus(&aria, CheckKind::Save(Ability::Dex)), 5);
        assert_eq!(bonus(&aria, CheckKind::Save(Ability::Cha)), -1);
        assert_eq!(bonus(&aria, CheckKind::Ability(Ability::Str)), 0);
    }

    #[test]
    fn parses_check_names() {
        assert_eq!(CheckKind::parse("dex save"), Some(CheckKind::Save(Ability::Dex)));
        assert_eq!(CheckKind::parse("Wisdom saving throw"), Some(CheckKind::Save(Ability::Wis)));
        assert_eq!(CheckKind::parse("stealth"), Some(CheckKind::Skill(Skill::Stealth)));
        assert_eq!(CheckKind::parse("STR"), Some(CheckKind::Ability(Ability::Str)));
        assert_eq!(CheckKind::parse("cooking"), None);
    }

    #[test]
    fn dc_decides_success() {
        let aria = chargen::sample_hero();
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..200 {
            let r = check(&aria, CheckKind::Skill(Skill::Stealth), Some(15), Adv::Normal, &mut rng);
            assert_eq!(r.success, Some(r.total >= 15));
            assert_eq!(r.total, r.natural as i32 + 5);
        }
    }
}
