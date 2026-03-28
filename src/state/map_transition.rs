//! # Map door transitions
//!
//! Polls door rects from `assets/maps/*/doors.json` each frame and determines whether the player
//! should transition to a new map.
//!
//! - **Walk-through** (`require_confirm: false`): triggers on the first frame the player enters
//!   the rect (edge detection via `prev_door_overlap`).
//! - **Confirm** (`require_confirm: true`): triggers on Space/Enter while inside the rect.
//! - **Story gate** (`require_state`): if the player's [`WorldState`] does not satisfy the
//!   condition, the transition is blocked and `deny_message` is shown as a transient overworld
//!   toast. See `docs/maps.md` for authoring.
//! - **Cooldown**: after a successful transition, a short [`DOOR_TRANSITION_COOLDOWN_SECS`]
//!   window suppresses overlap checks to prevent instant bounce-back.

use glam::Vec2;

use crate::config::MapDoor;
use crate::constants::DOOR_TRANSITION_COOLDOWN_SECS;
use crate::state::InputState;
use super::WorldState;

/// Result of polling doors: transition, blocked by story gate, or nothing this frame.
#[derive(Debug, Clone)]
pub enum DoorPollResult {
    None,
    Transition(MapDoor),
    Blocked {
        /// Empty if author omitted `deny_message` (a warning is logged).
        message: String,
    },
}

fn point_in_rect(pos: Vec2, rect: [f32; 4]) -> bool {
    let [x, y, w, h] = rect;
    pos.x >= x && pos.x < x + w && pos.y >= y && pos.y < y + h
}

fn first_overlapping_door_index(doors: &[MapDoor], pos: Vec2) -> Option<usize> {
    doors
        .iter()
        .enumerate()
        .find(|(_, d)| point_in_rect(pos, d.rect))
        .map(|(i, _)| i)
}

/// If a transition should occur this frame, returns the door config to apply. Updates overlap memory and cooldown.
///
/// [`DoorPollResult::Transition`] applies transition cooldown and clears overlap memory.
/// [`DoorPollResult::Blocked`] does not: the player can leave the door rect and retry without a long wait.
pub fn poll_map_door_transition(
    doors: &[MapDoor],
    player_pos: Vec2,
    input: &mut InputState,
    dt: f32,
    cooldown: &mut f32,
    prev_door_overlap: &mut Option<usize>,
    world_state: &WorldState,
) -> DoorPollResult {
    if *cooldown > 0.0 {
        *cooldown -= dt;
        if *cooldown <= 0.0 {
            *cooldown = 0.0;
            *prev_door_overlap = None;
        }
        return DoorPollResult::None;
    }

    let now = first_overlapping_door_index(doors, player_pos);

    let candidate = match now {
        Some(i) => {
            let door = doors[i].clone();
            if door.require_confirm {
                if input.confirm_pressed {
                    input.confirm_pressed = false;
                    Some(door)
                } else {
                    None
                }
            } else if prev_door_overlap != &Some(i) {
                Some(door)
            } else {
                None
            }
        }
        None => None,
    };

    *prev_door_overlap = now;

    let result = match candidate {
        Some(door) => {
            if let Some(ref req) = door.require_state {
                let req = req.trim();
                if !req.is_empty()
                    && !crate::dialogue::world_state_satisfies(Some(req), world_state)
                {
                    let msg = door.deny_message.as_deref().unwrap_or("").trim().to_string();
                    if msg.is_empty() {
                        log::warn!(
                            "door {:?} -> {:?}: require_state {:?} not satisfied but deny_message is missing or empty",
                            door.rect,
                            door.to_map,
                            req
                        );
                    }
                    DoorPollResult::Blocked { message: msg }
                } else {
                    DoorPollResult::Transition(door)
                }
            } else {
                DoorPollResult::Transition(door)
            }
        }
        None => DoorPollResult::None,
    };

    if matches!(result, DoorPollResult::Transition(_)) {
        *cooldown = DOOR_TRANSITION_COOLDOWN_SECS;
        *prev_door_overlap = None;
    }

    result
}
