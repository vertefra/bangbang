//! Resolve skill effects against entities with [`crate::ecs::Health`].

use hecs::{Entity, World};

use crate::ecs::{Health, Npc, Player, Transform};

use super::defs::{EffectOp, EffectStep, EffectTarget, SkillDef};

use crate::constants::NPC_INTERACT_RANGE;

/// Apply every effect in `skill` to `caster` / `opponent` entities. Fails if a target lacks `Health`.
pub fn apply_skill(
    skill: &SkillDef,
    world: &mut World,
    caster: Entity,
    opponent: Entity,
) -> Result<(), String> {
    for step in &skill.effects {
        apply_effect_step(step, world, caster, opponent)?;
    }
    Ok(())
}

fn apply_effect_step(
    step: &EffectStep,
    world: &mut World,
    caster: Entity,
    opponent: Entity,
) -> Result<(), String> {
    let target = match step.target {
        EffectTarget::Caster => caster,
        EffectTarget::Opponent => opponent,
    };
    let mut h = world
        .get::<&mut Health>(target)
        .map_err(|_| format!("skills: entity {:?} has no Health", target))?;
    match step.op {
        EffectOp::DealDamage => {
            h.current = (h.current - step.amount).max(0);
        }
        EffectOp::Heal => {
            h.current = (h.current + step.amount).min(h.max);
        }
    }
    Ok(())
}

/// Nearest NPC with `Health` within [`NPC_INTERACT_RANGE`](crate::constants::NPC_INTERACT_RANGE) of the player, if any.
pub fn nearest_opponent_entity(world: &World) -> Option<Entity> {
    let player_pos = world
        .query::<(&Player, &Transform)>()
        .iter()
        .next()
        .map(|(_, (_, t))| t.position)?;
    let mut best: Option<(Entity, f32)> = None;
    for (e, (_, t, _)) in world.query::<(&Npc, &Transform, &Health)>().iter() {
        let d = player_pos.distance(t.position);
        if d > NPC_INTERACT_RANGE {
            continue;
        }
        match best {
            None => best = Some((e, d)),
            Some((_, bd)) if d < bd => best = Some((e, d)),
            _ => {}
        }
    }
    best.map(|(e, _)| e)
}

pub fn player_entity(world: &World) -> Option<Entity> {
    world.query::<&Player>().iter().next().map(|(e, _)| e)
}
