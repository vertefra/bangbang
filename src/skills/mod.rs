//! Data-driven skills: JSON under `assets/skills/`, effect steps with explicit targets (`caster` / `opponent`).
//!
//! See [`defs::SkillDef`] and [`apply::apply_skill`].

mod apply;
mod backpack_runtime;
mod backpack_view;
mod defs;
mod registry;

pub use apply::{apply_skill, nearest_opponent_entity, player_entity};
pub use backpack_runtime::{apply_backpack_hotkey, cycle_equipped_weapon, seed_demo_backpack};
pub use backpack_view::{
    cycle_equipped_weapon_in_backpack, is_weapon_skill, normalize_equipped_weapon,
    passive_ids_in_order, weapon_ids_in_order,
};
pub use defs::{EffectOp, EffectStep, EffectTarget, SkillCategory, SkillDef};
pub use registry::SkillRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{Backpack, Health};
    use hecs::World;

    #[test]
    fn weapon_and_passive_partition_by_subcategory() {
        let registry = SkillRegistry::load_builtins().expect("load");
        let backpack = Backpack {
            permanent: vec!["sidearm".into(), "beer".into()],
            usable: vec![],
            equipped_weapon_id: Some("sidearm".into()),
        };
        assert_eq!(
            weapon_ids_in_order(&backpack, &registry),
            vec!["sidearm".to_string()]
        );
        assert_eq!(
            passive_ids_in_order(&backpack, &registry),
            vec!["beer".to_string()]
        );
    }

    #[test]
    fn normalize_equipped_snaps_to_first_weapon() {
        let registry = SkillRegistry::load_builtins().expect("load");
        let mut backpack = Backpack {
            permanent: vec!["sidearm".into()],
            usable: vec![],
            equipped_weapon_id: Some("not_a_weapon_id".into()),
        };
        normalize_equipped_weapon(&mut backpack, &registry);
        assert_eq!(backpack.equipped_weapon_id.as_deref(), Some("sidearm"));
    }

    #[test]
    fn cycle_with_single_weapon_no_ops() {
        let registry = SkillRegistry::load_builtins().expect("load");
        let mut backpack = Backpack {
            permanent: vec!["sidearm".into()],
            usable: vec![],
            equipped_weapon_id: Some("sidearm".into()),
        };
        cycle_equipped_weapon_in_backpack(&mut backpack, &registry, 1);
        assert_eq!(backpack.equipped_weapon_id.as_deref(), Some("sidearm"));
    }

    #[test]
    fn deal_damage_and_heal_round_trip() {
        let registry = SkillRegistry::load_builtins().expect("load");
        let weapon = registry.get("sidearm").expect("sidearm");
        let beer = registry.get("beer").expect("beer");

        let mut world = World::new();
        let a = world.spawn((Health {
            current: 10,
            max: 10,
        },));
        let b = world.spawn((Health {
            current: 10,
            max: 10,
        },));

        apply_skill(weapon, &mut world, a, b).expect("weapon");
        assert_eq!(
            *world.get::<&Health>(b).unwrap(),
            Health {
                current: 8,
                max: 10
            }
        );

        *world.get::<&mut Health>(a).unwrap() = Health {
            current: 5,
            max: 10,
        };
        apply_skill(beer, &mut world, a, a).expect("beer");
        assert_eq!(
            *world.get::<&Health>(a).unwrap(),
            Health {
                current: 8,
                max: 10
            }
        );
    }
}
