//! # World setup
//!
//! **High-level:** Populates the ECS world with the player and NPCs from map data.
//! Player gets `Facing` and `AnimationState` (for direction and idle/walk); NPCs get `Facing` only.
//! Called from `main` after loading a map and after map transitions.
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

use super::components::{
    AnimationState, Backpack, DoorMarker, Facing, Health, MapProp, Npc, Player, Sprite, SpriteSheet,
    Transform,
};
use crate::{assets, map_loader::MapData};

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
    let health = carryover.map(|c| c.health).unwrap_or(DEFAULT_ACTOR_HEALTH);

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
        world.spawn((
            DoorMarker,
            Transform {
                position: glam::Vec2::new(x + w * 0.5, y + h * 0.5),
                scale: glam::Vec2::new(w / fw, h / fh),
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
