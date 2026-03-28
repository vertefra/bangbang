//! # Backpack panel UI model
//!
//! Builds [`BackpackPanelLines`] from the player's ECS [`Backpack`](crate::ecs::Backpack) and
//! the [`SkillRegistry`](crate::skills::SkillRegistry). Three sections:
//!
//! - **Usable** — consumable skills with charge counts (e.g. Beer ×3).
//! - **Weapons** — permanent skills with `subcategory == "weapon"`, marking which is equipped.
//! - **Passives** — remaining permanent skills.
//!
//! Computed each frame in `App::update()` when the backpack is open, then passed immutably to the
//! GPU renderer for drawing. See `docs/skills.md` and `docs/ui.md`.

use crate::ecs::{Backpack, Player};
use hecs::World;

use crate::skills::SkillRegistry;

/// Slot data for a single item in the backpack panel.
#[derive(Debug, Clone)]
pub struct BackpackSlot {
    pub skill_id: String,
    pub label: String,
}

/// Weapon slot additionally tracks whether this weapon is equipped.
#[derive(Debug, Clone)]
pub struct BackpackWeaponSlot {
    pub skill_id: String,
    pub label: String,
    pub is_equipped: bool,
}

/// Lines and metadata for the three-section backpack (usable / weapons / passives).
#[derive(Debug, Clone)]
pub struct BackpackPanelLines {
    pub usable: Vec<BackpackSlot>,
    pub weapons: Vec<BackpackWeaponSlot>,
    pub passives: Vec<BackpackSlot>,
}

pub fn backpack_panel_lines(world: &World, registry: &SkillRegistry) -> BackpackPanelLines {
    let (usable_src, backpack) = {
        let mut q = world.query::<(&Player, &Backpack)>();
        let Some((_, (_, b))) = q.iter().next() else {
            return BackpackPanelLines {
                usable: Vec::new(),
                weapons: Vec::new(),
                passives: Vec::new(),
            };
        };
        (b.usable.clone(), b.clone())
    };

    let usable: Vec<BackpackSlot> = usable_src
        .iter()
        .map(|s| {
            let name = registry
                .get(&s.skill_id)
                .map(|d| d.name.as_str())
                .unwrap_or(s.skill_id.as_str());
            BackpackSlot {
                skill_id: s.skill_id.clone(),
                label: format!("{} ({})", name, s.charges),
            }
        })
        .collect();

    let equipped = backpack.equipped_weapon_id.as_deref();
    let weapon_ids = crate::skills::weapon_ids_in_order(&backpack, registry);
    let weapons: Vec<BackpackWeaponSlot> = weapon_ids
        .iter()
        .map(|id| {
            let name = registry
                .get(id)
                .map(|d| d.name.as_str())
                .unwrap_or(id.as_str());
            let is_equipped = equipped == Some(id.as_str());
            let label = if is_equipped {
                format!("{} [equipped]", name)
            } else {
                name.to_string()
            };
            BackpackWeaponSlot {
                skill_id: id.clone(),
                label,
                is_equipped,
            }
        })
        .collect();

    let passive_ids = crate::skills::passive_ids_in_order(&backpack, registry);
    let passives: Vec<BackpackSlot> = passive_ids
        .iter()
        .map(|id| {
            let name = registry
                .get(id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| id.clone());
            BackpackSlot {
                skill_id: id.clone(),
                label: name,
            }
        })
        .collect();

    BackpackPanelLines {
        usable,
        weapons,
        passives,
    }
}
