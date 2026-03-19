//! # Input state
//!
//! **High-level:** Holds current keyboard movement intent (up/down/left/right booleans). Updated from
//! winit key events in `main`; read by overworld update to move the player. Provides a normalized
//! direction vector for movement.

use glam::Vec2;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Movement intent from keyboard. One boolean per direction; `direction()` turns them into a Vec2.
/// `confirm_pressed` is set on Space/Enter press and cleared by the consumer (e.g. dialogue) after reading.
#[derive(Default)]
pub struct InputState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm_pressed: bool,
}

impl InputState {
    /// Normalized movement direction (unit length when diagonal). Zero vector if no keys held.
    ///
    /// **Rust:** `bool as i32` = 0 or 1; we subtract left from right, up from down, then normalize.
    pub fn direction(&self) -> Vec2 {
        let x = self.right as i32 as f32 - self.left as i32 as f32;
        let y = self.down as i32 as f32 - self.up as i32 as f32;
        let v = Vec2::new(x, y);
        if v.length_squared() > 0.0 {
            v.normalize()
        } else {
            Vec2::ZERO
        }
    }

    /// Update one direction flag from a winit key event. Called from `main` on `KeyboardInput`.
    ///
    /// **Rust:** `if let PhysicalKey::Code(code) = ...` destructures the enum; `match code { ... }`
    /// handles each key. `_ => {}` ignores other keys.
    pub fn apply_key(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::ArrowUp | KeyCode::KeyW => self.up = pressed,
                KeyCode::ArrowDown | KeyCode::KeyS => self.down = pressed,
                KeyCode::ArrowLeft | KeyCode::KeyA => self.left = pressed,
                KeyCode::ArrowRight | KeyCode::KeyD => self.right = pressed,
                KeyCode::Space | KeyCode::Enter => self.confirm_pressed = self.confirm_pressed || pressed,
                _ => {}
            }
        }
    }
}
