//! # ECS components
//!
//! **High-level:** Components are the "data" attached to entities in the ECS (Entity-Component-System)
//! model. Types: `Transform` (position/scale), `Sprite` (color), `Player` (marker), `Npc` (id, dialogue_line),
//! `Facing` (Direction for sprite row), `Direction`, `AnimationKind`, `AnimationState` (idle/walk, frame, timer).
//! The `hecs` crate stores these on entities; systems query by component type.

use glam::Vec2;

// -----------------------------------------------------------------------------
// Transform
// -----------------------------------------------------------------------------

/// World-space position and scale for an entity.
///
/// **Rust:** `#[derive(Copy, Clone, Debug)]` auto-implements traits:
/// - `Copy`: the type is copied by value (no move); can use after assignment.
/// - `Clone`: explicit `.clone()` is available.
/// - `Debug`: can be printed with `{:?}` for debugging.
#[derive(Copy, Clone, Debug)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
}

/// **Rust: `impl Default for T`** — implements the `Default` trait so you can call `Transform::default()`
/// or use `..Default::default()` in struct literals. Ensures new transforms start at origin, scale 1.
impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
        }
    }
}

// -----------------------------------------------------------------------------
// Sprite
// -----------------------------------------------------------------------------

/// Visual representation: RGBA color. Fallback when no sprite sheet is loaded.
#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub color: [f32; 4],
}

/// Character sprite sheet by id. Renderer draws from assets/characters/{id}/sheet.png; falls back to Sprite color if missing.
#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub character_id: String,
}

// -----------------------------------------------------------------------------
// Player
// -----------------------------------------------------------------------------

/// Marker component: identifies the single "player" entity. No data; used in queries like
/// `query::<(&Player, &Transform)>()` to find the entity that the camera follows and that
/// receives movement input.
///
/// **Rust:** A "unit struct" (no fields) is a type used only for tagging; it's zero-sized.
#[derive(Debug, Clone, Copy)]
pub struct Player;

// -----------------------------------------------------------------------------
// Npc
// -----------------------------------------------------------------------------

/// Static NPC that shows dialogue when the player approaches. conversation_id keys into assets/dialogue/{id}.json; dialogue_line is fallback when file missing.
#[derive(Debug, Clone)]
pub struct Npc {
    pub id: String,
    pub conversation_id: String,
    pub dialogue_line: String,
}

// -----------------------------------------------------------------------------
// Facing (for sprite sheet row / direction)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Down,
    Up,
    Left,
    Right,
}

/// Which way the entity is facing. Used by renderer to pick sprite row; overworld updates from movement.
#[derive(Debug, Clone, Copy)]
pub struct Facing(pub Direction);

impl Default for Facing {
    fn default() -> Self {
        Facing(Direction::Down)
    }
}

// -----------------------------------------------------------------------------
// Animation (for sprite sheet column / idle vs walk)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationKind {
    Idle,
    Walk,
}

/// Current animation and frame. Overworld sets kind (idle when still, walk when moving); timer drives frame_index.
#[derive(Debug, Clone, Copy)]
pub struct AnimationState {
    pub kind: AnimationKind,
    pub frame_index: u32,
    pub timer: f32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            kind: AnimationKind::Idle,
            frame_index: 0,
            timer: 0.0,
        }
    }
}
