//! # World setup
//!
//! **High-level:** Populates the ECS world with the player and NPCs from map data.
//! Player gets `Facing` and `AnimationState` (for direction and idle/walk); NPCs get `Facing` only.
//! Called once from `main` after loading the intro map.

use hecs::World;

use crate::map_loader::MapData;
use super::components::{
    AnimationState, Backpack, Facing, Health, Npc, Player, Sprite, SpriteSheet, Transform,
};

/// Spawn player at map's player_start and NPCs from map data.
pub fn setup_world(world: &mut World, map_data: &MapData) {
    world.spawn((
        Player,
        Transform {
            position: glam::Vec2::from_array(map_data.player_start),
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
        Backpack::default(),
        Health { current: 10, max: 10 },
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
            Sprite {
                color: npc.color,
            },
            SpriteSheet {
                character_id: character_id.clone(),
            },
            Facing::default(),
            Health { current: 10, max: 10 },
        ));
    }
}
