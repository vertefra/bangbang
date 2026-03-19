//! # Map loader
//!
//! Shared loader for map data: reads `assets/maps/{id}.map/map.json` and `assets/maps/{id}.map/npc.json`
//! (npc.json is an array of NPC refs resolved from `assets/npc/{id}.npc.json`).
//! Returns tilemap, NPCs, and player start. Used at startup and for future map-switch events.

use crate::config::{CharacterNpcConfig, MapNpcEntry, NpcConfig};
use crate::map::Tilemap;
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_PLAYER_START: [f32; 2] = [160.0, 160.0];

/// Everything needed to apply a map: tilemap, NPCs (ref id + config), and player spawn position.
#[derive(Debug, Clone)]
pub struct MapData {
    pub tilemap: Tilemap,
    pub npcs: Vec<(String, NpcConfig)>,
    pub player_start: [f32; 2],
}

#[derive(Debug, Deserialize)]
struct MapJson {
    width: u32,
    height: u32,
    tiles: Vec<u32>,
    tile_size: f32,
    #[serde(default)]
    player_start: Option<[f32; 2]>,
}

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Path to a map folder: `assets/maps/{id}.map` under the crate root.
fn map_dir(id: &str) -> PathBuf {
    assets_dir()
        .join("maps")
        .join(format!("{}.map", id))
}

/// Load map by id. Reads `assets/maps/{id}.map/map.json` and `assets/maps/{id}.map/npc.json`
/// (array of NPC refs); each ref loads `assets/npc/{id}.npc.json`. On missing/parse error, uses fallback.
pub fn load_map(id: &str) -> MapData {
    let dir = map_dir(id);
    let map_path = dir.join("map.json");
    let npc_path = dir.join("npc.json");

    let (tilemap, player_start) = match std::fs::read_to_string(&map_path) {
        Ok(data) => match serde_json::from_str::<MapJson>(&data) {
            Ok(m) => (
                Tilemap {
                    width: m.width,
                    height: m.height,
                    tiles: m.tiles,
                    tile_size: m.tile_size,
                },
                m.player_start.unwrap_or(DEFAULT_PLAYER_START),
            ),
            Err(_) => (fallback_tilemap(), DEFAULT_PLAYER_START),
        },
        Err(_) => (fallback_tilemap(), DEFAULT_PLAYER_START),
    };

    let npcs = match std::fs::read_to_string(&npc_path) {
        Ok(data) => match serde_json::from_str::<Vec<MapNpcEntry>>(&data) {
            Ok(entries) => load_npcs_from_map(&entries),
            Err(_) => default_npcs(),
        },
        Err(_) => default_npcs(),
    };

    MapData {
        tilemap,
        npcs,
        player_start,
    }
}

fn load_npcs_from_map(entries: &[MapNpcEntry]) -> Vec<(String, NpcConfig)> {
    let npc_dir = assets_dir().join("npc");
    let mut npcs = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(char_cfg) = load_character_npc(&npc_dir, &entry.id) {
            let conversation_id = char_cfg
                .conversation_id
                .unwrap_or_else(|| entry.id.clone());
            npcs.push((
                entry.id.clone(),
                NpcConfig {
                    position: entry.position,
                    scale: char_cfg.scale,
                    color: char_cfg.color,
                    conversation_id,
                    dialogue_line: char_cfg.dialogue_line,
                },
            ));
        }
    }
    if npcs.is_empty() {
        default_npcs()
    } else {
        npcs
    }
}

/// Load character config (scale, color, dialogue). Try `{id}.npc/config.json` then `{id}.npc.json`.
fn load_character_npc(npc_dir: &std::path::Path, id: &str) -> Option<CharacterNpcConfig> {
    let folder_config = npc_dir.join(format!("{}.npc", id)).join("config.json");
    let legacy_file = npc_dir.join(format!("{}.npc.json", id));
    let data = std::fs::read_to_string(&folder_config)
        .or_else(|_| std::fs::read_to_string(&legacy_file))
        .ok()?;
    serde_json::from_str::<CharacterNpcConfig>(&data).ok()
}

fn fallback_tilemap() -> Tilemap {
    const W: u32 = 20;
    const H: u32 = 15;
    const TILE: f32 = 32.0;
    let mut tiles = vec![0u32; (W * H) as usize];
    let idx = |x: u32, y: u32| (y * W + x) as usize;
    for x in 0..W {
        tiles[idx(x, 0)] = 1;
        tiles[idx(x, H - 1)] = 1;
    }
    for y in 0..H {
        tiles[idx(0, y)] = 1;
        tiles[idx(W - 1, y)] = 1;
    }
    for x in 5..=8 {
        tiles[idx(x, 5)] = 1;
        tiles[idx(x, 10)] = 1;
    }
    Tilemap {
        width: W,
        height: H,
        tiles,
        tile_size: TILE,
    }
}

fn default_npcs() -> Vec<(String, NpcConfig)> {
    vec![(
        "mom".into(),
        NpcConfig {
            position: [100.0, 100.0],
            scale: [0.5, 0.5],
            color: [0.2, 0.6, 1.0, 1.0],
            conversation_id: "mom".into(),
            dialogue_line: "Helloworld!".into(),
        },
    )]
}
