//! # Overworld logic
//!
//! **High-level:** One frame of overworld behaviour: read input, move the player entity, update
//! `Facing` (from movement direction) and `AnimationState` (idle when still, walk + frame advance when moving),
//! resolve collisions against the tilemap (tile 0 = walkable, else blocking). Queries entities with
//! `Player`, `Transform`, `Facing`, and `AnimationState`.

use crate::ecs::{AnimationKind, AnimationState, Direction, Facing, Npc, Player, Transform, World};
use crate::map::Tilemap;
use crate::state::InputState;
use crate::constants::NPC_INTERACT_RANGE;

const PLAYER_SPEED: f32 = 200.0;
const WALK_FRAMES: u32 = 4;
const WALK_FPS: f32 = 8.0;

fn direction_from_vec2(v: glam::Vec2) -> Direction {
    if v.y.abs() >= v.x.abs() {
        if v.y >= 0.0 {
            Direction::Down
        } else {
            Direction::Up
        }
    } else if v.x >= 0.0 {
        Direction::Right
    } else {
        Direction::Left
    }
}

/// Represents an interaction with an NPC in the overworld.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcInteraction {
    pub npc_id: String,
    pub conversation_id: String,
}

/// Overworld update: apply movement and tilemap collision to the player.
/// Returns `(trigger, near_npc)`: trigger is `Some(NpcInteraction)` when player is in range of an NPC,
/// near_npc is true when player is in range this frame (caller uses this to avoid re-triggering after closing dialogue).
pub fn update(
    world: &mut World,
    input: &InputState,
    dt: f32,
    tilemap: &Tilemap,
) -> (Option<NpcInteraction>, bool) {
    let dir = input.direction();
    for (_e, (_, transform, facing, anim)) in world.query_mut::<(
        &Player,
        &mut Transform,
        &mut Facing,
        &mut AnimationState,
    )>() {
        if dir.length_squared() > 0.0 {
            facing.0 = direction_from_vec2(dir);
            anim.kind = AnimationKind::Walk;
            anim.timer += dt;
            anim.frame_index = ((anim.timer * WALK_FPS) as u32) % WALK_FRAMES;
            let delta = dir * PLAYER_SPEED * dt;
            let new_pos = transform.position + delta;
            transform.position = resolve_collision(transform.position, new_pos, tilemap);
        } else {
            anim.kind = AnimationKind::Idle;
            anim.frame_index = 0;
            anim.timer = 0.0;
        }
    }

    let player_pos = match world.query::<(&Player, &Transform)>().iter().next() {
        Some((_, (_, t))) => t.position,
        None => return (None, false),
    };
    let mut near_npc = false;
    let mut trigger = None;
    for (_, (npc, transform)) in world.query::<(&Npc, &Transform)>().iter() {
        let d = player_pos.distance(transform.position);
        if d <= NPC_INTERACT_RANGE {
            near_npc = true;
            trigger.get_or_insert_with(|| {
                NpcInteraction {
                    npc_id: npc.id.clone(),
                    conversation_id: npc.conversation_id.clone(),
                }
            });
            break;
        }
    }
    (trigger, near_npc)
}

/// Resolve movement against tilemap: slide on X or Y if the new tile is blocking; if final tile blocks, revert to old.
///
/// **Rust:** We work in tile indices: `(pos / ts).floor() as u32`. Separately test X-step, Y-step, then final cell.
fn resolve_collision(old: glam::Vec2, new: glam::Vec2, tilemap: &Tilemap) -> glam::Vec2 {
    let ts = tilemap.tile_size;
    let mut out = new;
    let tx_new = (new.x / ts).floor() as u32;
    let ty_new = (new.y / ts).floor() as u32;
    let tx_old = (old.x / ts).floor() as u32;
    let ty_old = (old.y / ts).floor() as u32;
    if tilemap.is_blocking(tx_new, ty_old) {
        out.x = old.x;
    }
    if tilemap.is_blocking(tx_old, ty_new) {
        out.y = old.y;
    }
    let tx_f = (out.x / ts).floor() as u32;
    let ty_f = (out.y / ts).floor() as u32;
    if tilemap.is_blocking(tx_f, ty_f) {
        return old;
    }
    out
}
