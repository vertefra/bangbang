//! Seed backpack slots and handle overworld hotkeys (backpack open + number keys).

use hecs::World;

use crate::ecs::Backpack;

use super::apply::{apply_skill, nearest_opponent_entity, player_entity};
use super::backpack_view::{cycle_equipped_weapon_in_backpack, normalize_equipped_weapon};
use super::defs::SkillCategory;
use super::registry::SkillRegistry;

/// Grant demo skills: permanent sidearm, usable beer with charges from JSON.
pub fn seed_demo_backpack(world: &mut World, registry: &SkillRegistry) -> Result<(), String> {
    let player = player_entity(world).ok_or_else(|| "skills: no Player entity".to_string())?;
    let beer = registry
        .get("beer")
        .ok_or_else(|| "skills: missing beer definition".to_string())?;
    let charges = beer
        .charges_default
        .ok_or_else(|| "skills: beer must define charges_default".to_string())?;

    let mut backpack = world
        .get::<&mut Backpack>(player)
        .map_err(|_| "skills: player has no Backpack".to_string())?;
    backpack.permanent.clear();
    backpack.permanent.push("sidearm".to_string());
    backpack.equipped_weapon_id = Some("sidearm".to_string());
    backpack.usable.clear();
    backpack.usable.push(crate::ecs::UsableSkillStack {
        skill_id: "beer".to_string(),
        charges,
    });
    Ok(())
}

/// Cycle equipped weapon when backpack is open (e.g. Tab). `delta` +1 or -1.
pub fn cycle_equipped_weapon(world: &mut World, registry: &SkillRegistry, delta: i32) {
    let Some(player) = player_entity(world) else {
        eprintln!("skills: weapon cycle ignored — no player");
        return;
    };
    let Ok(mut backpack) = world.get::<&mut Backpack>(player) else {
        eprintln!("skills: weapon cycle ignored — player has no Backpack");
        return;
    };
    cycle_equipped_weapon_in_backpack(&mut backpack, registry, delta);
}

/// Grant a skill to the player by id. Idempotent — does not add duplicates to `permanent`.
/// Auto-equips the skill if it is a weapon and no weapon is currently equipped.
pub fn give_skill(
    world: &mut World,
    registry: &SkillRegistry,
    skill_id: &str,
) -> Result<(), String> {
    let def = registry
        .get(skill_id)
        .ok_or_else(|| format!("give_skill: unknown skill '{skill_id}'"))?;

    let player = player_entity(world)
        .ok_or_else(|| "give_skill: no Player entity in world".to_string())?;

    let mut backpack = world
        .get::<&mut Backpack>(player)
        .map_err(|_| "give_skill: player has no Backpack component".to_string())?;

    if !backpack.permanent.iter().any(|id| id == skill_id) {
        backpack.permanent.push(skill_id.to_string());
    }

    if def.subcategory == "weapon" && backpack.equipped_weapon_id.is_none() {
        backpack.equipped_weapon_id = Some(skill_id.to_string());
    }

    Ok(())
}

/// Backpack open: **1** = use equipped weapon skill (damage nearest NPC in range).
/// **2** = use first usable skill (beer → heal self, consume one charge).
pub fn apply_backpack_hotkey(world: &mut World, registry: &SkillRegistry, digit: u8) {
    let Some(player) = player_entity(world) else {
        eprintln!("skills: hotkey ignored — no player");
        return;
    };

    match digit {
        1 => {
            let skill_id = {
                let Ok(mut backpack) = world.get::<&mut Backpack>(player) else {
                    return;
                };
                normalize_equipped_weapon(&mut backpack, registry);
                backpack.equipped_weapon_id.clone()
            };
            let Some(skill_id) = skill_id else {
                eprintln!("skills: no weapon equipped (add a permanent skill with subcategory \"weapon\")");
                return;
            };
            let Some(skill) = registry.get(&skill_id) else {
                eprintln!("skills: unknown permanent skill {:?}", skill_id);
                return;
            };
            if skill.category != SkillCategory::Permanent {
                eprintln!("skills: {:?} is not permanent", skill_id);
                return;
            }
            let Some(opponent) = nearest_opponent_entity(world) else {
                eprintln!("skills: no opponent in range for {:?}", skill_id);
                return;
            };
            if let Err(e) = apply_skill(skill, world, player, opponent) {
                eprintln!("skills: {}", e);
            }
        }
        2 => {
            let (skill_id, new_charges) = {
                let Ok(backpack) = world.get::<&Backpack>(player) else {
                    return;
                };
                let Some(stack) = backpack.usable.first() else {
                    return;
                };
                if stack.charges == 0 {
                    return;
                }
                (stack.skill_id.clone(), stack.charges - 1)
            };
            let Some(skill) = registry.get(&skill_id) else {
                eprintln!("skills: unknown usable skill {:?}", skill_id);
                return;
            };
            if skill.category != SkillCategory::Usable {
                eprintln!("skills: {:?} is not usable", skill_id);
                return;
            }
            if let Err(e) = apply_skill(skill, world, player, player) {
                eprintln!("skills: {}", e);
                return;
            }
            if let Ok(mut backpack) = world.get::<&mut Backpack>(player) {
                if let Some(stack) = backpack.usable.first_mut() {
                    if stack.skill_id == skill_id {
                        stack.charges = new_charges;
                        if stack.charges == 0 {
                            backpack.usable.remove(0);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
