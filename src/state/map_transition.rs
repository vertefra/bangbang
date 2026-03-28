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
//!   window suppresses overlap checks to prevent instant bounce-back. When cooldown ends, overlap
//!   is initialized from the current position so standing in the destination door rect does not
//!   count as a new entry.

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
            // Spawn or standing still inside a walk-through rect: if we cleared overlap here, the
            // next frame would look like a fresh entry and re-trigger. Seed overlap from position.
            *prev_door_overlap = first_overlapping_door_index(doors, player_pos);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WorldState;

    fn test_door() -> MapDoor {
        MapDoor {
            rect: [192.0, 544.0, 96.0, 32.0],
            to_map: "other".into(),
            spawn: [0.0, 0.0],
            require_confirm: false,
            prop: None,
            require_state: None,
            deny_message: None,
        }
    }

    #[test]
    fn cooldown_end_inside_walk_through_does_not_retrigger() {
        let doors = vec![test_door()];
        let pos = Vec2::new(240.0, 560.0);
        assert!(point_in_rect(pos, doors[0].rect));

        let mut input = InputState::default();
        let mut cooldown = DOOR_TRANSITION_COOLDOWN_SECS;
        let mut prev: Option<usize> = None;
        let world = WorldState::default();

        let r = poll_map_door_transition(
            &doors,
            pos,
            &mut input,
            0.2,
            &mut cooldown,
            &mut prev,
            &world,
        );
        assert!(matches!(r, DoorPollResult::None));
        assert!(cooldown > 0.0);

        let r = poll_map_door_transition(
            &doors,
            pos,
            &mut input,
            1.0,
            &mut cooldown,
            &mut prev,
            &world,
        );
        assert!(matches!(r, DoorPollResult::None));
        assert_eq!(cooldown, 0.0);
        assert_eq!(prev, Some(0));

        let r = poll_map_door_transition(
            &doors,
            pos,
            &mut input,
            0.016,
            &mut cooldown,
            &mut prev,
            &world,
        );
        assert!(matches!(r, DoorPollResult::None));
    }

    #[test]
    fn walk_through_triggers_once_on_entry() {
        let doors = vec![test_door()];
        let outside = Vec2::new(240.0, 500.0);
        let inside = Vec2::new(240.0, 560.0);
        let mut input = InputState::default();
        let mut cooldown = 0.0;
        let mut prev: Option<usize> = None;
        let world = WorldState::default();

        poll_map_door_transition(
            &doors,
            outside,
            &mut input,
            0.016,
            &mut cooldown,
            &mut prev,
            &world,
        );
        assert_eq!(prev, None);

        let r = poll_map_door_transition(
            &doors,
            inside,
            &mut input,
            0.016,
            &mut cooldown,
            &mut prev,
            &world,
        );
        assert!(matches!(r, DoorPollResult::Transition(_)));
        assert!(cooldown > 0.0);
    }
}
