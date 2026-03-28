//! # Save / load game
//!
//! Persists overworld state to `paths::save_game_file()` (JSON). Used from the backpack
//! (**F5** save, **F9** load). See `docs/game.md`.

use crate::ecs::components::{Backpack, Direction, Facing, Health, Npc, Player, Transform};
use crate::ecs::world::{despawn_all_entities, setup_world, PlayerCarryover};
use crate::map_loader::{self, MapData};
use crate::state::WorldState;
use crate::state::WorldStateSnapshot;
use hecs::World;

pub const SAVE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveGameData {
    pub version: u32,
    pub map_id: String,
    pub player_position: [f32; 2],
    pub facing: Direction,
    pub backpack: Backpack,
    pub health: Health,
    pub world: WorldStateSnapshot,
    pub npc_health: Vec<NpcHealthEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NpcHealthEntry {
    pub id: String,
    pub current: i32,
    pub max: i32,
}

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Map(map_loader::MapLoadError),
    NoPlayer,
    MissingSaveFile,
    UnsupportedVersion(u32),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "{e}"),
            SaveError::Json(e) => write!(f, "{e}"),
            SaveError::Map(e) => write!(f, "{e}"),
            SaveError::NoPlayer => write!(f, "no player entity"),
            SaveError::MissingSaveFile => write!(f, "no save file found"),
            SaveError::UnsupportedVersion(v) => write!(f, "unsupported save version {v}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

/// Build a snapshot from the running game (overworld only; caller ensures valid state).
pub fn capture_save(
    world: &World,
    map_id: &str,
    world_state: &WorldState,
) -> Result<SaveGameData, SaveError> {
    let mut q = world.query::<(&Player, &Transform, &Backpack, &Health, &Facing)>();
    let Some((_, (_, t, backpack, health, facing))) = q.iter().next() else {
        return Err(SaveError::NoPlayer);
    };

    let mut npc_health = Vec::new();
    for (_, (npc, h)) in world.query::<(&Npc, &Health)>().iter() {
        npc_health.push(NpcHealthEntry {
            id: npc.id.clone(),
            current: h.current,
            max: h.max,
        });
    }

    Ok(SaveGameData {
        version: SAVE_FORMAT_VERSION,
        map_id: map_id.to_string(),
        player_position: [t.position.x, t.position.y],
        facing: facing.0,
        backpack: backpack.clone(),
        health: *health,
        world: world_state.to_snapshot(),
        npc_health,
    })
}

pub fn write_save_file(data: &SaveGameData) -> Result<(), SaveError> {
    if data.version != SAVE_FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion(data.version));
    }
    let path = crate::paths::save_game_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(data).map_err(SaveError::Json)?;
    std::fs::write(&path, json)?;
    log::info!("wrote save to {}", path.display());
    Ok(())
}

pub fn read_save_file() -> Result<SaveGameData, SaveError> {
    let path = crate::paths::save_game_file();
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SaveError::MissingSaveFile
        } else {
            SaveError::Io(e)
        }
    })?;
    let data: SaveGameData = serde_json::from_str(&raw).map_err(SaveError::Json)?;
    if data.version != SAVE_FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion(data.version));
    }
    Ok(data)
}

/// Rebuild ECS + return loaded map data for `MapContext`. Clears and respawns the world.
pub fn restore_world_from_save(data: &SaveGameData, world: &mut World, world_state: &mut WorldState) -> Result<MapData, SaveError> {
    if data.version != SAVE_FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion(data.version));
    }
    let map_data = map_loader::load_map(&data.map_id).map_err(SaveError::Map)?;
    despawn_all_entities(world);
    let carry = PlayerCarryover {
        backpack: data.backpack.clone(),
        health: data.health,
    };
    setup_world(world, &map_data, data.player_position, Some(carry));
    let Some(player) = crate::skills::player_entity(world) else {
        return Err(SaveError::NoPlayer);
    };
    if let Ok(mut facing) = world.get::<&mut Facing>(player) {
        facing.0 = data.facing;
    }
    world_state.restore_from_snapshot(data.world.clone());

    for entry in &data.npc_health {
        for (e, npc) in world.query::<&Npc>().iter() {
            if npc.id == entry.id {
                if let Ok(mut h) = world.get::<&mut Health>(e) {
                    *h = Health {
                        current: entry.current,
                        max: entry.max,
                    };
                }
                break;
            }
        }
    }

    Ok(map_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Player;
    use crate::map_loader::load_map;
    use hecs::World;

    #[test]
    fn round_trip_capture_write_read_restore() {
        let map_id = "mumhome.secondFloor";
        let map_data = load_map(map_id).expect("test map");
        let mut world = World::new();
        setup_world(&mut world, &map_data, map_data.player_start, None);
        let mut ws = WorldState::new();
        ws.set_flag("test_flag");

        let snap = capture_save(&world, map_id, &ws).expect("capture");
        let json = serde_json::to_string(&snap).expect("serde");
        let parsed: SaveGameData = serde_json::from_str(&json).expect("parse");

        let mut world2 = World::new();
        let mut ws2 = WorldState::new();
        restore_world_from_save(&parsed, &mut world2, &mut ws2).expect("restore");

        assert!(ws2.has_flag("test_flag"));
        let mut q = world2.query::<(&Player, &Transform)>();
        let (_, (_, t)) = q.iter().next().expect("player");
        assert_eq!(t.position.x, snap.player_position[0]);
    }
}
