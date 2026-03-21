//! # Input state
//!
//! **High-level:** Holds current keyboard movement intent (up/down/left/right booleans). Updated from
//! winit key events in `main`; read by overworld update to move the player. Provides a normalized
//! direction vector for movement.

use glam::Vec2;
use winit::event::{ElementState, KeyEvent, Modifiers};
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
    pub backpack_pressed: bool,
    /// Digit 1–9 pressed this frame (for backpack skill hotkeys). Consumed by `take_skill_hotkey_digit`.
    skill_hotkey_digit: Option<u8>,
    /// Tab / Shift+Tab weapon cycle while backpack is open. Consumed by `take_weapon_cycle_step`.
    weapon_cycle_step: Option<i32>,
}

impl InputState {
    /// Take pending backpack number key (1–9), if any, and clear it.
    pub fn take_skill_hotkey_digit(&mut self) -> Option<u8> {
        self.skill_hotkey_digit.take()
    }

    /// Take pending equipped-weapon cycle delta (+1 Tab, -1 Shift+Tab), if any, and clear it.
    pub fn take_weapon_cycle_step(&mut self) -> Option<i32> {
        self.weapon_cycle_step.take()
    }

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
        self.apply_key_with_modifiers(event, &Modifiers::default());
    }

    /// Same as `apply_key`, but Tab / Shift+Tab need active modifier state from the window event.
    pub fn apply_key_with_modifiers(&mut self, event: &KeyEvent, mods: &Modifiers) {
        let pressed = event.state == ElementState::Pressed;
        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::ArrowUp | KeyCode::KeyW => self.up = pressed,
                KeyCode::ArrowDown | KeyCode::KeyS => self.down = pressed,
                KeyCode::ArrowLeft | KeyCode::KeyA => self.left = pressed,
                KeyCode::ArrowRight | KeyCode::KeyD => self.right = pressed,
                KeyCode::Space | KeyCode::Enter => self.confirm_pressed = self.confirm_pressed || pressed,
                KeyCode::KeyB => self.backpack_pressed = self.backpack_pressed || pressed,
                KeyCode::Digit1
                | KeyCode::Digit2
                | KeyCode::Digit3
                | KeyCode::Digit4
                | KeyCode::Digit5
                | KeyCode::Digit6
                | KeyCode::Digit7
                | KeyCode::Digit8
                | KeyCode::Digit9 => {
                    if pressed && !event.repeat {
                        self.skill_hotkey_digit = Some(match code {
                            KeyCode::Digit1 => 1,
                            KeyCode::Digit2 => 2,
                            KeyCode::Digit3 => 3,
                            KeyCode::Digit4 => 4,
                            KeyCode::Digit5 => 5,
                            KeyCode::Digit6 => 6,
                            KeyCode::Digit7 => 7,
                            KeyCode::Digit8 => 8,
                            KeyCode::Digit9 => 9,
                            _ => unreachable!(),
                        });
                    }
                }
                KeyCode::Tab => {
                    if pressed && !event.repeat {
                        let delta = if mods.state().shift_key() { -1 } else { 1 };
                        self.weapon_cycle_step = Some(delta);
                    }
                }
                _ => {}
            }
        }
    }
}
