//! Map door overlap and transition polling (see `assets/maps/*/doors.json`).

use glam::Vec2;

use crate::config::MapDoor;
use crate::constants::DOOR_TRANSITION_COOLDOWN_SECS;
use crate::state::InputState;

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
pub fn poll_map_door_transition(
    doors: &[MapDoor],
    player_pos: Vec2,
    input: &mut InputState,
    dt: f32,
    cooldown: &mut f32,
    prev_door_overlap: &mut Option<usize>,
) -> Option<MapDoor> {
    if *cooldown > 0.0 {
        *cooldown -= dt;
        if *cooldown <= 0.0 {
            *cooldown = 0.0;
            *prev_door_overlap = None;
        }
        return None;
    }

    let now = first_overlapping_door_index(doors, player_pos);

    let out = match now {
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

    if out.is_some() {
        *cooldown = DOOR_TRANSITION_COOLDOWN_SECS;
        *prev_door_overlap = None;
    }

    out
}
