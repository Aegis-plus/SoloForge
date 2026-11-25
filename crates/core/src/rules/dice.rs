//! Dice expressions: `1d20+5`, `2d6 + 1d4 - 1`, `4d6kh3`, `2d20kl1`, `d%`,
//! and `1d20 adv` / `1d20 dis`. All rolls come from the app's RNG — the GM
//! never invents a number.

use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub sign: i32,
    pub count: u32,
    pub sides: u32,
    pub keep: Option<Keep>,
    pub constant: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Keep {
    Highest(u32),
    Lowest(u32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermResult {
    pub sign: i32,
    pub sides: u32,
    pub dice: Vec<u32>,
    pub kept: Vec<bool>,
    pub constant: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollResult {
    pub expr: String,
    pub total: i32,
    pub terms: Vec<TermResult>,
}

impl RollResult {
    /// "1d20 (14) + 5", "2d20kh1 (3, 17) + 7", "2d8 (6, 3) + 3"
    pub fn breakdown(&self) -> String {
        let mut out = String::new();
        for (i, t) in self.terms.iter().enumerate() {
            let op = if t.sign < 0 { " − " } else if i == 0 { "" } else { " + " };
            if i == 0 && t.sign < 0 {
                out.push('−');
            } else {
                out.push_str(op);
            }
            if t.sides == 0 {
                out.push_str(&t.constant.to_string());
            } else {
                let shown: Vec<String> = t
                    .dice
                    .iter()
                    .zip(&t.kept)
                    .map(|(d, k)| if *k || t.dice.len() == 1 { d.to_string() } else { format!("~{d}~") })
                    .collect();
                let sides = if t.sides == 100 { "%".to_string() } else { t.sides.to_string() };
                out.push_str(&format!("{}d{} ({})", t.dice.len(), sides, shown.join(", ")));
            }
        }
        out
    }

    /// The first kept d20 in the roll, for natural 1/20 detection.
    pub fn natural_d20(&self) -> Option<u32> {
        let t = self.terms.iter().find(|t| t.sides == 20)?;
        t.dice.iter().zip(&t.kept).find(|(_, k)| **k).map(|(d, _)| *d)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceError(pub String);
impl std::fmt::Display for DiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for DiceError {}

pub fn parse(expr: &str) -> Result<Vec<Term>, DiceError> {
    let mut s = expr.trim().to_lowercase().replace('−', "-");
    let mut adv: Option<bool> = None;
    for (word, a) in [(" advantage", true), (" adv", true), (" disadvantage", false), (" dis", false)] {
        if s.ends_with(word) {
            s.truncate(s.len() - word.len());
            adv = Some(a);
            break;
        }
    }
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() {
        return Err(DiceError("empty dice expression".into()));
    }
    let mut terms = vec![];
    let mut sign = 1;
    let mut cur = String::new();
    let flush = |cur: &mut String, sign: i32, terms: &mut Vec<Term>| -> Result<(), DiceError> {
        if cur.is_empty() {
            return Err(DiceError("dangling operator".into()));
        }
        terms.push(parse_term(cur, sign)?);
        cur.clear();
        Ok(())
    };
    for (i, c) in s.chars().enumerate() {
        if (c == '+' || c == '-') && i > 0 {
            flush(&mut cur, sign, &mut terms)?;
            sign = if c == '-' { -1 } else { 1 };
        } else if c == '-' && i == 0 {
            sign = -1;
        } else {
            cur.push(c);
        }
    }
    flush(&mut cur, sign, &mut terms)?;
    if let Some(a) = adv {
        let t = terms
            .iter_mut()
            .find(|t| t.sides == 20 && t.count == 1)
            .ok_or_else(|| DiceError("advantage needs a single d20".into()))?;
        t.count = 2;
        t.keep = Some(if a { Keep::Highest(1) } else { Keep::Lowest(1) });
    }
    let dice: u32 = terms.iter().map(|t| t.count).sum();
    if dice > 100 {
        return Err(DiceError("too many dice (max 100)".into()));
    }
    Ok(terms)
}

fn parse_term(t: &str, sign: i32) -> Result<Term, DiceError> {
    if let Some(dpos) = t.find('d') {
        let count: u32 = if dpos == 0 { 1 } else { t[..dpos].parse().map_err(|_| DiceError(format!("bad dice count in “{t}”")))? };
        let rest = &t[dpos + 1..];
        let (sides_s, keep) = if let Some(k) = rest.find("kh") {
            (&rest[..k], Some(Keep::Highest(rest[k + 2..].parse().map_err(|_| DiceError("bad kh".into()))?)))
        } else if let Some(k) = rest.find("kl") {
            (&rest[..k], Some(Keep::Lowest(rest[k + 2..].parse().map_err(|_| DiceError("bad kl".into()))?)))
        } else {
            (rest, None)
        };
        let sides: u32 = if sides_s == "%" { 100 } else { sides_s.parse().map_err(|_| DiceError(format!("bad die size in “{t}”")))? };
        if count == 0 || !(2..=1000).contains(&sides) {
            return Err(DiceError(format!("“{t}” is not a die I can roll")));
        }
        Ok(Term { sign, count, sides, keep, constant: 0 })
    } else {
        let c: i32 = t.parse().map_err(|_| DiceError(format!("“{t}” is not a number or dice")))?;
        Ok(Term { sign, count: 0, sides: 0, keep: None, constant: c })
    }
}

pub fn roll_terms<R: Rng + ?Sized>(expr: &str, terms: &[Term], rng: &mut R) -> RollResult {
    let mut total = 0;
    let mut out = vec![];
    for t in terms {
        if t.sides == 0 {
            total += t.sign * t.constant;
            out.push(TermResult { sign: t.sign, sides: 0, dice: vec![], kept: vec![], constant: t.constant });
            continue;
        }
        let dice: Vec<u32> = (0..t.count).map(|_| rng.gen_range(1..=t.sides)).collect();
        let mut kept = vec![true; dice.len()];
        if let Some(k) = t.keep {
            let mut idx: Vec<usize> = (0..dice.len()).collect();
            match k {
                Keep::Highest(_) => idx.sort_by(|a, b| dice[*b].cmp(&dice[*a])),
                Keep::Lowest(_) => idx.sort_by(|a, b| dice[*a].cmp(&dice[*b])),
            }
            let n = match k {
                Keep::Highest(n) | Keep::Lowest(n) => n as usize,
            };
            kept = vec![false; dice.len()];
            for i in idx.into_iter().take(n) {
                kept[i] = true;
            }
        }
        let sum: i32 = dice.iter().zip(&kept).filter(|(_, k)| **k).map(|(d, _)| *d as i32).sum();
        total += t.sign * sum;
        out.push(TermResult { sign: t.sign, sides: t.sides, dice, kept, constant: 0 });
    }
    RollResult { expr: expr.to_string(), total, terms: out }
}

pub fn roll<R: Rng + ?Sized>(expr: &str, rng: &mut R) -> Result<RollResult, DiceError> {
    let terms = parse(expr)?;
    Ok(roll_terms(expr, &terms, rng))
}

/// Double the dice of every damage term (critical hit), keep modifiers.
pub fn crit_expr(expr: &str) -> Result<String, DiceError> {
    let terms = parse(expr)?;
    let mut out = String::new();
    for (i, t) in terms.iter().enumerate() {
        if i > 0 || t.sign < 0 {
            out.push(if t.sign < 0 { '-' } else { '+' });
        }
        if t.sides == 0 {
            out.push_str(&t.constant.to_string());
        } else {
            out.push_str(&format!("{}d{}", t.count * 2, t.sides));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn parses_common_expressions() {
        assert_eq!(parse("1d20+5").unwrap().len(), 2);
        assert_eq!(parse("2d6 + 1d4 - 1").unwrap().len(), 3);
        let t = parse("4d6kh3").unwrap();
        assert_eq!(t[0].keep, Some(Keep::Highest(3)));
        let t = parse("1d20+7 adv").unwrap();
        assert_eq!((t[0].count, t[0].keep), (2, Some(Keep::Highest(1))));
        let t = parse("d20 dis").unwrap();
        assert_eq!(t[0].keep, Some(Keep::Lowest(1)));
        assert_eq!(parse("d%").unwrap()[0].sides, 100);
        assert_eq!(parse("-1").unwrap()[0].sign, -1);
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["", "d", "1d1", "abc", "2d6+", "1000d6", "1d6 adv"] {
            assert!(parse(bad).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn totals_stay_in_range() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..2000 {
            let r = roll("2d6+3", &mut rng).unwrap();
            assert!((5..=15).contains(&r.total));
            let r = roll("4d6kh3", &mut rng).unwrap();
            assert!((3..=18).contains(&r.total));
            assert_eq!(r.terms[0].kept.iter().filter(|k| **k).count(), 3);
        }
    }

    #[test]
    fn advantage_keeps_the_higher_die() {
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..500 {
            let r = roll("1d20 adv", &mut rng).unwrap();
            let d = &r.terms[0].dice;
            assert_eq!(r.total as u32, *d.iter().max().unwrap());
            assert_eq!(r.natural_d20(), Some(r.total as u32));
        }
    }

    #[test]
    fn d20_is_roughly_uniform() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut counts = [0u32; 21];
        for _ in 0..40_000 {
            counts[roll("1d20", &mut rng).unwrap().total as usize] += 1;
        }
        for c in &counts[1..] {
            assert!((1700..2300).contains(c), "{counts:?}");
        }
    }

    #[test]
    fn crit_doubles_dice_not_modifiers() {
        assert_eq!(crit_expr("1d8+3").unwrap(), "2d8+3");
        assert_eq!(crit_expr("2d6+1d4-1").unwrap(), "4d6+2d4-1");
    }

    #[test]
    fn breakdown_reads_well() {
        let mut rng = StdRng::seed_from_u64(3);
        let r = roll("1d20+5", &mut rng).unwrap();
        let b = r.breakdown();
        assert!(b.starts_with("1d20 (") && b.ends_with(" + 5"), "{b}");
    }
}
