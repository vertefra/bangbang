//! # ECS (Entity-Component-System) module
//!
//! **High-level:** This module owns the game world (entities + components) and their setup.
//! - `components`: defines `Transform`, `Sprite`, `Player`, `Npc`, `Facing`, `Direction`, `AnimationKind`, `AnimationState`.
//! - `world`: `setup_world()` spawns initial entities (player with Facing + AnimationState, NPCs with Facing).
//! - We re-export types so callers can do `bangbang::ecs::{World, Transform, setup_world}`.
//!
//! **Rust:** `pub use X` re-exports `X` from this module, so `ecs::World` refers to `hecs::World`.

pub mod components;
pub mod world;

pub use components::{AnimationKind, AnimationState, Direction, Facing, Npc, Player, Sprite, SpriteSheet, Transform};
pub use hecs::World;
pub use world::setup_world;
