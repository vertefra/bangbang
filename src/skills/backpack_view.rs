//! Partition permanent skills into weapons vs passives and keep `equipped_weapon_id` consistent.

use crate::ecs::Backpack;

use super::registry::SkillRegistry;

pub fn is_weapon_skill(registry: &SkillRegistry, id: &str) -> bool {
    registry
        .get(id)
        .map(|d| d.subcategory == "weapon")
        .unwrap_or(false)
}

/// Permanent skill ids that are weapons, in backpack order.
pub fn weapon_ids_in_order(backpack: &Backpack, registry: &SkillRegistry) -> Vec<String> {
    backpack
        .permanent
        .iter()
        .filter(|id| is_weapon_skill(registry, id.as_str()))
        .cloned()
        .collect()
}

/// Permanent skill ids that are not weapons, in backpack order.
pub fn passive_ids_in_order(backpack: &Backpack, registry: &SkillRegistry) -> Vec<String> {
    backpack
        .permanent
        .iter()
        .filter(|id| !is_weapon_skill(registry, id.as_str()))
        .cloned()
        .collect()
}

/// Clears equipped if no weapons; if equipped is missing or invalid, sets to first weapon in order.
pub fn normalize_equipped_weapon(backpack: &mut Backpack, registry: &SkillRegistry) {
    let weapons = weapon_ids_in_order(backpack, registry);
    if weapons.is_empty() {
        backpack.equipped_weapon_id = None;
        return;
    }
    let ok = backpack
        .equipped_weapon_id
        .as_ref()
        .is_some_and(|id| weapons.iter().any(|w| w == id));
    if !ok {
        backpack.equipped_weapon_id = Some(weapons[0].clone());
    }
}

/// Cycle equipped weapon by `delta` (+1 / -1). No-op if fewer than two weapons.
pub fn cycle_equipped_weapon_in_backpack(
    backpack: &mut Backpack,
    registry: &SkillRegistry,
    delta: i32,
) {
    normalize_equipped_weapon(backpack, registry);
    let weapons = weapon_ids_in_order(backpack, registry);
    if weapons.len() < 2 {
        return;
    }
    let n = weapons.len() as i32;
    let current = backpack
        .equipped_weapon_id
        .as_deref()
        .unwrap_or(weapons[0].as_str());
    let idx = weapons
        .iter()
        .position(|w| w == current)
        .unwrap_or(0) as i32;
    let next = (idx + delta).rem_euclid(n) as usize;
    backpack.equipped_weapon_id = Some(weapons[next].clone());
}
