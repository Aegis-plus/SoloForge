//! Initiative, turn order, attacks and damage.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{dice, signed, Ability};
use crate::world::{Actor, Attack, Combat, Combatant, RollCard, RollLine, World};

pub fn roll_initiative<R: Rng + ?Sized>(world: &World, actor_ids: &[String], rng: &mut R) -> Combat {
    let mut order: Vec<(Combatant, i32)> = actor_ids
        .iter()
        .filter_map(|id| world.actor(id))
        .map(|a| {
            let dex = a.abilities.modifier(Ability::Dex);
            let init = rng.gen_range(1..=20) + dex;
            (Combatant { actor_id: a.id.clone(), initiative: init, hidden: false }, dex)
        })
        .collect();
    order.sort_by(|(a, ad), (b, bd)| b.initiative.cmp(&a.initiative).then(bd.cmp(ad)));
    Combat { round: 1, turn: 0, order: order.into_iter().map(|(c, _)| c).collect(), name: String::new() }
}

/// Advance to the next combatant who is still standing. Returns the new current actor id.
pub fn next_turn(world: &mut World) -> Option<String> {
    let alive: Vec<bool> = {
        let c = world.combat.as_ref()?;
        c.order.iter().map(|cb| world.actor(&cb.actor_id).map(|a| !a.is_down()).unwrap_or(false)).collect()
    };
    let c = world.combat.as_mut()?;
    if c.order.is_empty() {
        return None;
    }
    if !alive.iter().any(|a| *a) {
        return None;
    }
    loop {
        c.turn += 1;
        if c.turn >= c.order.len() {
            c.turn = 0;
            c.round += 1;
        }
        if alive[c.turn] {
            break;
        }
    }
    Some(c.order[c.turn].actor_id.clone())
}

pub fn current_actor(world: &World) -> Option<&Actor> {
    let c = world.combat.as_ref()?;
    world.actor(&c.order.get(c.turn)?.actor_id)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttackOutcome {
    pub hit: bool,
    pub crit: bool,
    pub damage: i32,
    pub card: RollCard,
}

pub fn resolve_attack<R: Rng + ?Sized>(attacker: &Actor, attack: &Attack, target: &Actor, adv: super::checks::Adv, rng: &mut R) -> AttackOutcome {
    let suffix = match adv {
        super::checks::Adv::Normal => "",
        super::checks::Adv::Advantage => " adv",
        super::checks::Adv::Disadvantage => " dis",
    };
    let to_hit_expr = format!("1d20{}{}", if attack.to_hit >= 0 { "+" } else { "" }, attack.to_hit) + suffix;
    let r = dice::roll(&to_hit_expr, rng).expect("valid attack roll");
    let nat = r.natural_d20().unwrap_or(0);
    let crit = nat == 20;
    let hit = crit || (nat != 1 && r.total >= target.ac);
    let mut lines = vec![RollLine { label: "To Hit".into(), detail: format!("{} vs AC {}", r.breakdown(), target.ac), total: r.total, kind: "to_hit".into() }];
    let mut damage = 0;
    if hit {
        let expr = if crit { dice::crit_expr(&attack.damage).unwrap_or_else(|_| attack.damage.clone()) } else { attack.damage.clone() };
        if let Ok(d) = dice::roll(&expr, rng) {
            damage = d.total.max(1);
            let ty = if attack.damage_type.is_empty() { String::new() } else { format!(" {}", attack.damage_type) };
            lines.push(RollLine { label: "Damage".into(), detail: format!("{}{}", d.breakdown(), ty), total: damage, kind: "damage".into() });
        }
    }
    let outcome = if crit { "crit" } else if nat == 1 { "fumble" } else if hit { "success" } else { "fail" };
    let card = RollCard {
        title: format!("{} → {}", attack.name, target.name),
        actor: attacker.name.clone(),
        lines,
        outcome: outcome.into(),
        note: String::new(),
        by_gm: false,
    };
    AttackOutcome { hit, crit, damage, card }
}

/// Apply damage: temp HP first; solo rule — minions drop on any hit.
/// Returns (hp_before, hp_after).
pub fn apply_damage(actor: &mut Actor, amount: i32) -> (i32, i32) {
    let before = actor.hp;
    let mut left = amount.max(0);
    if actor.temp_hp > 0 {
        let used = actor.temp_hp.min(left);
        actor.temp_hp -= used;
        left -= used;
    }
    actor.hp = if actor.minion && amount > 0 { 0 } else { (actor.hp - left).max(0) };
    if actor.hp == 0 && !actor.conditions.iter().any(|c| c == "Down") {
        actor.conditions.push("Down".into());
        actor.concentrating = None;
    }
    (before, actor.hp)
}

pub fn heal(actor: &mut Actor, amount: i32) -> (i32, i32) {
    let before = actor.hp;
    actor.hp = (actor.hp + amount.max(0)).min(actor.max_hp);
    if actor.hp > 0 {
        actor.conditions.retain(|c| c != "Down");
    }
    (before, actor.hp)
}

pub fn hp_summary(a: &Actor) -> String {
    format!("{}: {}/{} HP, AC {}, {}", a.name, a.hp, a.max_hp, a.ac, if a.is_down() { "down".to_string() } else { format!("init {}", signed(a.abilities.modifier(Ability::Dex))) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{chargen, compendium};
    use rand::{rngs::StdRng, SeedableRng};

    fn table() -> World {
        let mut w = World::new("t", "");
        w.actors.push(chargen::sample_hero());
        let mut g = compendium::monster("goblin").unwrap();
        g.name = "Goblin A".into();
        w.actors.push(g);
        let mut g2 = compendium::monster("goblin").unwrap();
        g2.name = "Goblin B".into();
        w.actors.push(g2);
        w
    }

    #[test]
    fn initiative_is_sorted_and_turns_skip_the_fallen() {
        let mut w = table();
        let ids: Vec<String> = w.actors.iter().map(|a| a.id.clone()).collect();
        let mut rng = StdRng::seed_from_u64(2);
        let c = roll_initiative(&w, &ids, &mut rng);
        assert!(c.order.windows(2).all(|p| p[0].initiative >= p[1].initiative));
        w.combat = Some(c);
        let first = w.combat.as_ref().unwrap().order[1].actor_id.clone();
        w.actor_mut(&first).unwrap().hp = 0;
        let next = next_turn(&mut w).unwrap();
        assert_ne!(next, first);
        for _ in 0..5 {
            let id = next_turn(&mut w).unwrap();
            assert_ne!(id, first);
        }
        assert!(w.combat.as_ref().unwrap().round >= 2);
    }

    #[test]
    fn minions_drop_on_any_hit_and_temp_hp_soaks() {
        let mut g = compendium::monster("goblin").unwrap();
        assert!(g.minion);
        assert_eq!(apply_damage(&mut g, 1), (7, 0));
        assert!(g.conditions.contains(&"Down".to_string()));
        let mut aria = chargen::sample_hero();
        aria.temp_hp = 5;
        assert_eq!(apply_damage(&mut aria, 8), (24, 21));
        assert_eq!(aria.temp_hp, 0);
        assert_eq!(heal(&mut aria, 100), (21, 31));
    }

    #[test]
    fn crits_always_hit_and_double_dice() {
        let aria = chargen::sample_hero();
        let mut target = compendium::monster("goblin").unwrap();
        target.ac = 99;
        let mut rng = StdRng::seed_from_u64(0);
        let mut saw_crit = false;
        for _ in 0..400 {
            let o = resolve_attack(&aria, &aria.attacks[0], &target, super::super::checks::Adv::Normal, &mut rng);
            if o.crit {
                saw_crit = true;
                assert!(o.hit && o.damage >= 2 + 3);
                assert!(o.card.lines[1].detail.starts_with("2d8"));
            } else {
                assert!(!o.hit);
            }
        }
        assert!(saw_crit);
    }
}
