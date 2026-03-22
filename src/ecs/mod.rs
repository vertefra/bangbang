//! # ECS (Entity-Component-System) module
//!
//! **High-level:** This module owns the game world (entities + components) and their setup.
//! - `components`: defines `Transform`, `Sprite`, `Player`, `Npc`, `MapProp`, `Health`, `Backpack`, `Facing`, `Direction`, `AnimationKind`, `AnimationState`.
//! - `world`: `setup_world(world, map_data, player_spawn, carryover)` spawns player (Facing + AnimationState), NPCs, and map props from [`crate::map_loader::MapData`]; `take_player_carryover` / `despawn_all_entities` support map transitions. NPC asset layout: `docs/npc.md`; props: `docs/maps.md` (`props.json`).
//! - We re-export types so callers can do `bangbang::ecs::{World, Transform, setup_world}`.
//!
//! **Rust:** `pub use X` re-exports `X` from this module, so `ecs::World` refers to `hecs::World`.

pub mod components;
pub mod world;

pub use components::{
    AnimationKind, AnimationState, Backpack, Direction, DoorMarker, Facing, Health, MapProp, Npc,
    Player, Sprite, SpriteSheet, Transform, UsableSkillStack,
};
pub use hecs::World;
pub use world::{despawn_all_entities, setup_world, take_player_carryover, PlayerCarryover};
