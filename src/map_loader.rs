//! # Map loader
//!
//! Shared loader for map data: reads `assets/maps/{id}.map/map.json` and `assets/maps/{id}.map/npc.json`
//! (npc.json is an array of NPC refs resolved from `assets/npc/{id}.npc.json`).
//! Returns tilemap, NPCs, and player start. Used at startup and for future map-switch events.
//!
//! `map.json` `tiles` may be a flat row-major array or a matrix (`[[row0...], [row1...], ...]`).
//! Required `tile_palette` in `map.json` loads `assets/tile_palettes/{id}.json` (`color` + `walkable` per tile id).

use crate::assets::LoadedSheet;
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
    pub tileset: Option<LoadedSheet>,
}

/// `tiles` in JSON may be a flat row-major array or a matrix (array of rows, top to bottom).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TilesJson {
    Flat(Vec<u32>),
    Matrix(Vec<Vec<u32>>),
}

fn tiles_row_major(width: u32, height: u32, tiles: TilesJson) -> Option<Vec<u32>> {
    let expected = (width * height) as usize;
    match tiles {
        TilesJson::Flat(v) => (v.len() == expected).then_some(v),
        TilesJson::Matrix(rows) => {
            if rows.len() != height as usize {
                return None;
            }
            let mut out = Vec::with_capacity(expected);
            for row in rows {
                if row.len() != width as usize {
                    return None;
                }
                out.extend(row);
            }
            Some(out)
        }
    }
}

#[derive(Debug, Deserialize)]
struct MapJson {
    width: u32,
    height: u32,
    tiles: TilesJson,
    tile_size: f32,
    #[serde(default)]
    player_start: Option<[f32; 2]>,
    #[serde(default)]
    tileset: Option<String>,
    /// Pixel size of one cell in `assets/tiles/{tileset}.png` (overrides `{tileset}.json`). Set this
    /// on Wang / multi-tile sheets so the correct grid is used even if the sidecar JSON is wrong.
    #[serde(default)]
    tileset_tile_size: Option<u32>,
    #[serde(default)]
    tileset_draw: Option<crate::map::TilesetDraw>,
    tile_palette: String,
}

use crate::paths;

/// Path to a map folder: `assets/maps/{id}.map` under the crate root.
fn map_dir(id: &str) -> PathBuf {
    paths::asset_root()
        .join("maps")
        .join(format!("{}.map", id))
}

/// Load `assets/tile_palettes/{id}.json`. Invalid or missing file → `None`.
fn load_tile_palette(id: &str) -> Option<crate::map::TilePalette> {
    let path = crate::paths::asset_root().join("tile_palettes").join(format!("{id}.json"));
    let data = std::fs::read_to_string(&path).ok()?;
    crate::map::TilePalette::from_json_str(&data).ok()
}

#[derive(Debug)]
pub enum MapLoadError {
    Io(std::io::Error, PathBuf),
    Json(serde_json::Error, PathBuf),
    InvalidTiles(u32, u32, PathBuf),
    MissingPalette(String),
}

impl std::fmt::Display for MapLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e, p) => write!(f, "IO error at {}: {}", p.display(), e),
            Self::Json(e, p) => write!(f, "JSON error at {}: {}", p.display(), e),
            Self::InvalidTiles(w, h, p) => write!(f, "Invalid tiles ({}x{}) at {}", w, h, p.display()),
            Self::MissingPalette(id) => write!(f, "Missing tile palette: {}", id),
        }
    }
}

/// Load map by id. Reads `assets/maps/{id}.map/map.json` and `assets/maps/{id}.map/npc.json`
/// (array of NPC refs); each ref loads `assets/npc/{id}.npc.json`.
pub fn load_map(id: &str) -> Result<MapData, MapLoadError> {
    let dir = map_dir(id);
    let map_path = dir.join("map.json");
    let npc_path = dir.join("npc.json");

    let map_data = std::fs::read_to_string(&map_path)
        .map_err(|e| MapLoadError::Io(e, map_path.clone()))?;
    let m: MapJson = serde_json::from_str(&map_data)
        .map_err(|e| MapLoadError::Json(e, map_path.clone()))?;

    let tiles = tiles_row_major(m.width, m.height, m.tiles)
        .ok_or_else(|| MapLoadError::InvalidTiles(m.width, m.height, map_path.clone()))?;

    let tile_palette = load_tile_palette(&m.tile_palette)
        .ok_or_else(|| MapLoadError::MissingPalette(m.tile_palette.clone()))?;

    let tilemap = Tilemap {
        width: m.width,
        height: m.height,
        tiles,
        tile_size: m.tile_size,
        tileset_draw: m.tileset_draw,
        tile_palette,
    };

    let tileset = m.tileset
        .as_deref()
        .and_then(|id| crate::assets::load_tileset(id, m.tileset_tile_size));

    let npcs = match std::fs::read_to_string(&npc_path) {
        Ok(data) => {
            let entries: Vec<MapNpcEntry> = serde_json::from_str(&data)
                .map_err(|e| MapLoadError::Json(e, npc_path.clone()))?;
            load_npcs_from_map(&entries)
        }
        Err(_) => {
            log::warn!("NPC config missing for map {}: {}; using default NPCs", id, npc_path.display());
            default_npcs()
        }
    };

    Ok(MapData {
        tilemap,
        npcs,
        player_start: m.player_start.unwrap_or(DEFAULT_PLAYER_START),
        tileset,
    })
}

fn load_npcs_from_map(entries: &[MapNpcEntry]) -> Vec<(String, NpcConfig)> {
    let npc_dir = paths::asset_root().join("npc");
    let mut npcs = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Ok(char_cfg) = load_character_npc(&npc_dir, &entry.id) {
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
fn load_character_npc(npc_dir: &std::path::Path, id: &str) -> Result<CharacterNpcConfig, MapLoadError> {
    let folder_config = npc_dir.join(format!("{}.npc", id)).join("config.json");
    let legacy_file = npc_dir.join(format!("{}.npc.json", id));

    if let Ok(data) = std::fs::read_to_string(&folder_config) {
        return serde_json::from_str::<CharacterNpcConfig>(&data)
            .map_err(|e| MapLoadError::Json(e, folder_config));
    }

    let data = std::fs::read_to_string(&legacy_file)
        .map_err(|e| MapLoadError::Io(e, legacy_file.clone()))?;
    log::warn!("DEPRECATION: {id}.npc.json used; migrate to {id}.npc/config.json");
    serde_json::from_str::<CharacterNpcConfig>(&data)
        .map_err(|e| MapLoadError::Json(e, legacy_file))
}

fn default_npcs() -> Vec<(String, NpcConfig)> {
    vec![(
        "mom".into(),
        NpcConfig {
            position: [100.0, 100.0],
            scale: [0.5, 0.5],
            color: [0.2, 0.6, 1.0, 1.0],
            conversation_id: "mom".into(),
        },
    )]
}

/// Prints to stderr where this binary was built from and how the tileset decoded. If the path is
/// not the repo you are editing, you are running the wrong executable; if the grid is `1×1`,
/// every map cell will show the same sheet tile.
pub fn log_startup_tilemap_diagnostics(data: &MapData) {
    let intro = paths::asset_root().join("maps/intro.map/map.json");
    eprintln!("[bangbang] Asset Root = {}", paths::asset_root().display());
    eprintln!(
        "[bangbang] intro map.json present: {} ({})",
        intro.display(),
        if intro.exists() { "yes" } else { "NO" }
    );
    match &data.tileset {
        Some(s) => eprintln!(
            "[bangbang] tileset: {}×{} cells, {}×{} px per cell",
            s.cols, s.rows, s.frame_width, s.frame_height
        ),
        None => eprintln!("[bangbang] tileset: NOT LOADED"),
    }
    eprintln!("[bangbang] tileset_draw: {:?}", data.tilemap.tileset_draw);
}

#[cfg(test)]
mod tests {
    #[test]
    fn intro_map_defines_tileset_draw_for_binary_grid() {
        let d = crate::map_loader::load_map("intro").expect("intro map should load");
        let td = d.tilemap.tileset_draw.as_ref().expect(
            "intro.map must set tileset_draw: logical 0/1 are collision ids, not wang sheet indices",
        );
        assert!(td.wang_autotile);
        assert_eq!(td.floor, 6);
    }

    #[test]
    fn intro_tileset_is_four_by_four_at_32px() {
        let d = crate::map_loader::load_map("intro").expect("intro map should load");
        let s = d.tileset.expect("intro loads farwest_interior");
        assert_eq!((s.cols, s.rows, s.frame_width), (4, 4, 32));
    }
}
