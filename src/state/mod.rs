//! # State module
//!
//! **High-level:** Game state and input. `AppState` = current mode (Overworld/Dialogue/Duel);
//! `InputState` = keyboard movement flags. Overworld handles movement and tilemap collision.

pub mod app;
pub mod input;
pub mod map_transition;
pub mod overworld;
pub mod world;

pub use app::AppState;
pub use input::InputState;
pub use world::WorldState;
