//! # World setup
//!
//! **High-level:** Populates the ECS world with the player and NPCs from map data.
//! Player gets `Facing` and `AnimationState` (for direction and idle/walk); NPCs get `Facing` only.
//! Called from `main` after loading a map and after map transitions.
//!
//! ## Player HP vs NPC HP
//!
//! The player spawns at **5 / 5** LP on first load ([`DEFAULT_PLAYER_HEALTH`]). NPCs use **10 / 10**
//! ([`DEFAULT_ACTOR_HEALTH`]). After a map change, [`PlayerCarryover`] restores the player’s prior
//! backpack and [`Health`] when present; otherwise the player gets [`DEFAULT_PLAYER_HEALTH`] again.
//!
//! ## NPC spawn
//!
//! For each `(character_id, npc)` in [`crate::map_loader::MapData::npcs`], spawns an entity with
//! [`Npc`], [`Transform`], [`Sprite`], [`SpriteSheet`] (`character_id` = map id string),
//! [`Facing`], and [`Health`]. See `docs/npc.md`.
//!
//! For each [`crate::config::MapPropEntry`] in [`crate::map_loader::MapData::props`], resolves the
//! prop asset id and spawns [`MapProp`], [`Transform`], [`Sprite`], [`SpriteSheet`] (no [`Npc`], no dialogue).
//!
//! For each [`crate::config::MapDoor`] with a `prop`, spawns [`DoorMarker`] + textured quad;
//! missing / `"none"` skips the entity (transition still uses `rect`).

use hecs::World;

use glam::Vec2;

use super::components::{
    AnimationKind, AnimationState, Backpack, Direction, DoorMarker, Facing, Health, MapProp, Npc,
    Player, SceneActor, SceneActorMotion, Sprite, SpriteSheet, Transform,
};
use crate::constants::SCENE_ACTOR_SCALE_MULTIPLIER;
use crate::scene::{SceneDef, SceneStep};
use crate::{assets, map_loader::MapData};

/// Starting LP for the player when there is no [`PlayerCarryover`] (first load or missing player).
const DEFAULT_PLAYER_HEALTH: Health = Health {
    current: 5,
    max: 5,
};

/// Starting HP for spawned NPCs. Placeholder — later, per-character values may load from config.
const DEFAULT_ACTOR_HEALTH: Health = Health {
    current: 10,
    max: 10,
};

/// Backpack + HP preserved across `despawn_all_entities` / `setup_world` when changing maps.
#[derive(Debug, Clone)]
pub struct PlayerCarryover {
    pub backpack: Backpack,
    pub health: Health,
}

/// Read player inventory and HP before tearing down the world for a map change.
pub fn take_player_carryover(world: &mut World) -> Option<PlayerCarryover> {
    let (e, _) = world.query::<&Player>().iter().next()?;
    let backpack = (*world.get::<&Backpack>(e).ok()?).clone();
    let health = *world.get::<&Health>(e).ok()?;
    Some(PlayerCarryover { backpack, health })
}

/// Remove every entity (player, NPCs, etc.). Used when switching maps.
pub fn despawn_all_entities(world: &mut World) {
    let entities: Vec<hecs::Entity> = world.iter().map(|er| er.entity()).collect();
    for e in entities {
        let _ = world.despawn(e);
    }
}

/// Spawn player at `player_spawn`, NPCs, map props, and door markers from `map_data`.
pub fn setup_world(
    world: &mut World,
    map_data: &MapData,
    player_spawn: [f32; 2],
    carryover: Option<PlayerCarryover>,
) {
    let backpack = carryover
        .as_ref()
        .map(|c| c.backpack.clone())
        .unwrap_or_default();
    let health = carryover
        .map(|c| c.health)
        .unwrap_or(DEFAULT_PLAYER_HEALTH);

    world.spawn((
        Player,
        Transform {
            position: glam::Vec2::from_array(player_spawn),
            scale: glam::Vec2::new(1.0, 1.0),
        },
        Sprite {
            color: [1.0, 0.5, 0.2, 1.0],
        },
        SpriteSheet {
            character_id: "player".into(),
        },
        Facing::default(),
        AnimationState::default(),
        backpack,
        health,
    ));

    for (character_id, npc) in &map_data.npcs {
        world.spawn((
            Npc {
                id: character_id.clone(),
                conversation_id: npc.conversation_id.clone(),
            },
            Transform {
                position: glam::Vec2::from_array(npc.position),
                scale: glam::Vec2::from_array(npc.scale),
            },
            Sprite { color: npc.color },
            SpriteSheet {
                character_id: character_id.clone(),
            },
            Facing::default(),
            DEFAULT_ACTOR_HEALTH,
        ));
    }

    for prop in &map_data.props {
        let Some(sheet_id) = assets::resolve_map_prop_sheet_id(&prop.id) else {
            log::warn!("map prop '{}' not found under assets/props/", prop.id);
            continue;
        };
        world.spawn((
            MapProp {
                id: prop.id.clone(),
            },
            Transform {
                position: glam::Vec2::from_array(prop.position),
                scale: glam::Vec2::from_array(prop.scale),
            },
            Sprite {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            SpriteSheet {
                character_id: sheet_id,
            },
        ));
    }

    for door in &map_data.doors {
        let Some(prop_id) = door.prop_id() else {
            continue;
        };
        let Some(sheet_id) = assets::resolve_door_prop_sheet_id(prop_id) else {
            log::warn!("door prop '{}' not found under assets/props/", prop_id);
            continue;
        };
        let Some(sheet) = assets::load_character_sheet(&sheet_id) else {
            log::warn!("door prop '{}' could not load sheet.png", sheet_id);
            continue;
        };
        let fw = sheet.frame_width as f32;
        let fh = sheet.frame_height as f32;
        if fw <= 0.0 || fh <= 0.0 {
            log::warn!("door prop '{}' has invalid frame size {}x{}", sheet_id, fw, fh);
            continue;
        }
        let [x, y, w, h] = door.rect;
        // Non-uniform scale distorts door art (e.g. 48×48 sheet stretched into 64×32). Fit inside
        // the rect with uniform scale and align the sprite bottom to the rect bottom (floor line).
        let uniform = (w / fw).min(h / fh);
        let drawn_h = fh * uniform;
        world.spawn((
            DoorMarker,
            Transform {
                position: glam::Vec2::new(x + w * 0.5, y + h - drawn_h * 0.5),
                scale: glam::Vec2::splat(uniform),
            },
            Sprite {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            SpriteSheet {
                character_id: sheet_id,
            },
        ));
    }
}

// --- Scene actors (cutscene characters in the world) -------------------------------------------

const SCENE_WALK_FPS: f32 = 8.0;
const SCENE_WALK_FRAMES: u32 = 4;

/// Converts a display name like `"Bank Owner"` to the asset folder id `bankOwner`.
pub fn speaker_to_asset_id(speaker: &str) -> String {
    let parts: Vec<&str> = speaker.split_whitespace().filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return "unknown".to_string();
    }
    let mut out = parts[0].to_lowercase();
    for p in &parts[1..] {
        let mut ch = p.chars();
        if let Some(f) = ch.next() {
            for c in f.to_uppercase() {
                out.push(c);
            }
            out.extend(ch.flat_map(|c| c.to_lowercase()));
        }
    }
    out
}

/// Remove all [`SceneActor`] entities (e.g. when a scene ends or the map reloads).
pub fn despawn_scene_actors(world: &mut World) {
    let to_remove: Vec<_> = world
        .query::<&SceneActor>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    for e in to_remove {
        let _ = world.despawn(e);
    }
}

fn direction_toward(from: Vec2, to: Vec2) -> Direction {
    direction_from_vec2(to - from)
}

/// Facing from a direction vector (e.g. cutscene movement velocity).
fn direction_from_vec2(d: Vec2) -> Direction {
    if d.length_squared() < 1e-4 {
        return Direction::Down;
    }
    if d.y.abs() >= d.x.abs() {
        if d.y >= 0.0 {
            Direction::Down
        } else {
            Direction::Up
        }
    } else if d.x >= 0.0 {
        Direction::Right
    } else {
        Direction::Left
    }
}

/// Spawn or keep the cutscene character for the current dialogue step (matched by `SpriteSheet::character_id`).
pub fn sync_scene_actor_for_step(world: &mut World, scene_def: &SceneDef, step: usize) {
    let desired_id: Option<String> = match scene_def.steps.get(step) {
        Some(SceneStep::Dialogue { portrait, speaker, .. }) => Some(
            portrait
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string)
                .unwrap_or_else(|| speaker_to_asset_id(speaker)),
        ),
        _ => None,
    };

    let Some(desired_id) = desired_id else {
        despawn_scene_actors(world);
        return;
    };

    for (_, (_, ss)) in world.query::<(&SceneActor, &SpriteSheet)>().iter() {
        if ss.character_id == desired_id {
            return;
        }
    }

    despawn_scene_actors(world);

    let cfg = match crate::map_loader::load_character_npc_config(&desired_id) {
        Ok(c) => c,
        Err(e) => {
            log::error!("scene actor '{}': {}", desired_id, e);
            return;
        }
    };

    let player_pos = match world.query::<(&Player, &Transform)>().iter().next() {
        Some((_, (_, t))) => t.position,
        None => {
            log::error!("scene actor: no player");
            return;
        }
    };

    // Spawn offset + flee velocity: Silas starts west and runs east across view; others stand by the player.
    let (spawn_offset, velocity) = match desired_id.as_str() {
        "silas" => (
            Vec2::new(-340.0, -28.0),
            Vec2::new(220.0, 32.0),
        ),
        "bankOwner" => (Vec2::new(150.0, -52.0), Vec2::ZERO),
        _ => {
            let side = if step % 2 == 0 { 1.0 } else { -1.0 };
            (
                Vec2::new(side * 140.0, -48.0),
                Vec2::ZERO,
            )
        }
    };
    let pos = player_pos + spawn_offset;
    let scale = Vec2::from_array(cfg.scale) * SCENE_ACTOR_SCALE_MULTIPLIER;
    let initial_facing = if velocity.length_squared() > 1.0 {
        direction_from_vec2(velocity)
    } else {
        direction_toward(pos, player_pos)
    };

    world.spawn((
        SceneActor,
        SceneActorMotion { velocity },
        Transform {
            position: pos,
            scale,
        },
        Sprite {
            color: cfg.color,
        },
        SpriteSheet {
            character_id: desired_id,
        },
        Facing(initial_facing),
        AnimationState {
            kind: AnimationKind::Walk,
            frame_index: 0,
            timer: 0.0,
        },
    ));
}

/// Move cutscene actors by [`SceneActorMotion::velocity`] each frame.
pub fn tick_scene_actor_motion(world: &mut World, dt: f32) {
    for (_, (_, transform, motion)) in
        world.query_mut::<(&SceneActor, &mut Transform, &SceneActorMotion)>()
    {
        transform.position += motion.velocity * dt;
    }
}

/// Advance walk animation for cutscene characters (same cadence as player walk).
pub fn tick_scene_actor_animations(world: &mut World, dt: f32) {
    for (_, (_, anim)) in world.query_mut::<(&SceneActor, &mut AnimationState)>() {
        anim.kind = AnimationKind::Walk;
        anim.timer += dt;
        anim.frame_index = ((anim.timer * SCENE_WALK_FPS) as u32) % SCENE_WALK_FRAMES;
    }
}

/// Update cutscene facing: movers face their velocity; stationary actors face the player.
pub fn face_scene_actors(world: &mut World) {
    let player_pos = match world.query::<(&Player, &Transform)>().iter().next() {
        Some((_, (_, t))) => t.position,
        None => return,
    };
    for (_, (_, transform, facing, motion)) in world
        .query_mut::<(&SceneActor, &Transform, &mut Facing, &SceneActorMotion)>()
    {
        facing.0 = if motion.velocity.length_squared() > 25.0 {
            direction_from_vec2(motion.velocity)
        } else {
            direction_toward(transform.position, player_pos)
        };
    }
}
