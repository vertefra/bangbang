use crate::ecs::{Backpack, Player};
use hecs::World;

use crate::skills::SkillRegistry;

/// Lines and metadata for the three-section backpack (usable / weapons / passives).
#[derive(Debug, Clone)]
pub struct BackpackPanelLines {
    pub usable: Vec<String>,
    /// Display label and whether this row is the equipped weapon.
    pub weapons: Vec<(String, bool)>,
    pub passives: Vec<String>,
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

    let usable: Vec<String> = usable_src
        .iter()
        .map(|s| {
            let name = registry
                .get(&s.skill_id)
                .map(|d| d.name.as_str())
                .unwrap_or(s.skill_id.as_str());
            format!("{} ({})", name, s.charges)
        })
        .collect();

    let equipped = backpack.equipped_weapon_id.as_deref();
    let weapon_ids = crate::skills::weapon_ids_in_order(&backpack, registry);
    let weapons: Vec<(String, bool)> = weapon_ids
        .iter()
        .map(|id| {
            let name = registry
                .get(id)
                .map(|d| d.name.as_str())
                .unwrap_or(id.as_str());
            let is_eq = equipped == Some(id.as_str());
            let label = if is_eq {
                format!("{} *", name)
            } else {
                name.to_string()
            };
            (label, is_eq)
        })
        .collect();

    let passive_ids = crate::skills::passive_ids_in_order(&backpack, registry);
    let passives: Vec<String> = passive_ids
        .iter()
        .map(|id| {
            registry
                .get(id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| id.clone())
        })
        .collect();

    BackpackPanelLines {
        usable,
        weapons,
        passives,
    }
}
