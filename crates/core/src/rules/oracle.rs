//! The solo oracle: yes/no questions with a likelihood, “and/but” twists,
//! and random events on doubles (driven by Tension 1–9).

use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Likelihood {
    Impossible,
    Unlikely,
    EvenOdds,
    Likely,
    Certain,
}

impl Likelihood {
    pub fn parse(s: &str) -> Likelihood {
        match s.trim().to_lowercase().replace([' ', '-', '_'], "").as_str() {
            "impossible" | "nearlyimpossible" => Likelihood::Impossible,
            "unlikely" | "doubtful" => Likelihood::Unlikely,
            "likely" | "probable" => Likelihood::Likely,
            "certain" | "nearlycertain" | "verylikely" => Likelihood::Certain,
            _ => Likelihood::EvenOdds,
        }
    }
    /// Roll ≤ threshold on d100 means yes.
    pub fn threshold(self) -> u32 {
        match self {
            Likelihood::Impossible => 10,
            Likelihood::Unlikely => 30,
            Likelihood::EvenOdds => 50,
            Likelihood::Likely => 70,
            Likelihood::Certain => 90,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Likelihood::Impossible => "Nearly Impossible",
            Likelihood::Unlikely => "Unlikely",
            Likelihood::EvenOdds => "Even Odds",
            Likelihood::Likely => "Likely",
            Likelihood::Certain => "Nearly Certain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleAnswer {
    pub roll: u32,
    pub likelihood: Likelihood,
    pub answer: String,
    pub yes: bool,
    pub event: Option<String>,
}

const FOCUS: [&str; 10] = [
    "A remote event",
    "An NPC acts",
    "A new NPC appears",
    "The thread moves toward its goal",
    "The thread moves away from its goal",
    "An old enemy acts",
    "The hero gains an advantage",
    "The hero faces a setback",
    "Something ambiguous happens",
    "The current thread closes",
];
const ACTIONS: [&str; 20] = [
    "betray", "reveal", "protect", "pursue", "abandon", "summon", "bargain", "hide", "break", "awaken", "guide", "warn",
    "steal", "mourn", "ignite", "trap", "heal", "lure", "corrupt", "return",
];
const SUBJECTS: [&str; 20] = [
    "a secret", "an ally", "the past", "a debt", "a stranger", "an oath", "the dead", "a map", "the weather", "a beast",
    "a relic", "a home", "a rumour", "a rival", "the law", "a child", "a door", "a fire", "a song", "the road",
];

pub fn ask<R: Rng + ?Sized>(likelihood: Likelihood, tension: u8, rng: &mut R) -> OracleAnswer {
    let roll: u32 = rng.gen_range(1..=100);
    let t = likelihood.threshold();
    let yes = roll <= t;
    // Exceptional results: the top/bottom fifth of each side.
    let answer = if yes {
        if roll <= t / 5 { "Yes, and…" } else { "Yes" }
    } else if roll > 100 - (100 - t) / 5 {
        "No, and…"
    } else if roll <= t + (100 - t) / 5 {
        "No, but…"
    } else {
        "No"
    };
    let doubles = roll.is_multiple_of(11) && roll < 100;
    let event = if doubles && (roll / 11) as u8 <= tension.clamp(1, 9) {
        Some(random_event(rng))
    } else {
        None
    };
    OracleAnswer { roll, likelihood, answer: answer.into(), yes, event }
}

pub fn random_event<R: Rng + ?Sized>(rng: &mut R) -> String {
    let f = FOCUS[rng.gen_range(0..FOCUS.len())];
    let a = ACTIONS[rng.gen_range(0..ACTIONS.len())];
    let s = SUBJECTS[rng.gen_range(0..SUBJECTS.len())];
    format!("{f} — {a} {s}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn likelier_means_more_yes() {
        let mut rng = StdRng::seed_from_u64(5);
        let rate = |l: Likelihood, rng: &mut StdRng| (0..5000).filter(|_| ask(l, 5, rng).yes).count();
        let (u, e, l) = (rate(Likelihood::Unlikely, &mut rng), rate(Likelihood::EvenOdds, &mut rng), rate(Likelihood::Likely, &mut rng));
        assert!(u < e && e < l, "{u} {e} {l}");
        assert!((1300..1700).contains(&u));
    }

    #[test]
    fn events_only_on_doubles_within_tension() {
        let mut rng = StdRng::seed_from_u64(11);
        for _ in 0..3000 {
            let a = ask(Likelihood::EvenOdds, 3, &mut rng);
            if a.event.is_some() {
                assert!(a.roll % 11 == 0 && a.roll / 11 <= 3);
            }
        }
    }

    #[test]
    fn parses_likelihood_words() {
        assert_eq!(Likelihood::parse("Likely"), Likelihood::Likely);
        assert_eq!(Likelihood::parse("even odds"), Likelihood::EvenOdds);
        assert_eq!(Likelihood::parse("nearly-certain"), Likelihood::Certain);
    }
}
