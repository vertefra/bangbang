//! # Map loader
//!
//! Shared loader for map data: reads `assets/maps/{id}.map/map.json`, optional `npc.json`, optional `doors.json`, and optional `props.json`.
//! Returns tilemap, NPCs, and player start. Used at startup and for map transitions.
//!
//! ## NPC pipeline
//!
//! 1. Parse `npc.json` into [`crate::config::MapNpcEntry`] rows.
//! 2. For each row, load [`crate::config::CharacterNpcConfig`] via `load_character_npc` (`{id}.npc/config.json`, else legacy `{id}.npc.json`).
//! 3. Build [`crate::config::NpcConfig`]: `position` from the map entry; `scale` / `color` / `conversation_id` from the character file (`conversation_id` defaults to map `id`).
//!
//! **Props:** `props.json` → [`crate::config::MapPropEntry`] list (missing file = empty). Preferred
//! sheets: `assets/props/{id}.prop/sheet.png`.
//!
//! See repository `docs/npc.md` for NPC authoring; `docs/maps.md` for `props.json`.
//!
//! ## Tiles
//!
//! `map.json` `tiles` may be a flat row-major array or a matrix (`[[row0...], [row1...], ...]`).
//! Required `tile_palette` in `map.json` loads `assets/tile_palettes/{id}.json` (`color` + `walkable` per tile id).

use crate::assets::LoadedSheet;
use crate::config::{CharacterNpcConfig, MapDoor, MapNpcEntry, MapPropEntry, MapSceneTrigger, NpcConfig};
use crate::map::Tilemap;
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_PLAYER_START: [f32; 2] = [160.0, 160.0];

/// Everything needed to apply a map: tilemap, NPCs, props, doors, scene triggers, and default player spawn.
#[derive(Debug, Clone)]
pub struct MapData {
    pub tilemap: Tilemap,
    /// `(character_id, merged_config)` pairs for [`crate::ecs::world::setup_world`]. `character_id` is the map `npc.json` `id` (also `SpriteSheet.character_id`).
    pub npcs: Vec<(String, NpcConfig)>,
    /// Static props from `props.json` (buildings, etc.); sheets under `assets/props/{id}.prop/`
    /// by convention, with legacy folder fallback.
    pub props: Vec<MapPropEntry>,
    pub doors: Vec<MapDoor>,
    /// Scene proximity triggers from `scenes.json`. Missing file → empty (not an error).
    pub scene_triggers: Vec<MapSceneTrigger>,
    pub player_start: [f32; 2],
    pub tileset: Option<LoadedSheet>,
}

#[derive(Debug, Deserialize)]
struct SparseRect {
    id: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Deserialize)]
struct SparseTilesJson {
    #[serde(default)]
    fill: u32,
    #[serde(default)]
    perimeter: Option<u32>,
    #[serde(default)]
    rects: Vec<SparseRect>,
}

/// `tiles` in JSON may be a flat row-major array, a matrix, or a sparse object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TilesJson {
    Flat(Vec<u32>),
    Matrix(Vec<Vec<u32>>),
    Sparse(SparseTilesJson),
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
        TilesJson::Sparse(s) => {
            let mut out = vec![s.fill; expected];
            if let Some(pid) = s.perimeter {
                for x in 0..width {
                    out[0 * width as usize + x as usize] = pid;
                    out[(height as usize - 1) * width as usize + x as usize] = pid;
                }
                for y in 0..height {
                    out[y as usize * width as usize + 0] = pid;
                    out[y as usize * width as usize + (width as usize - 1)] = pid;
                }
            }
            for r in s.rects {
                for ry in r.y..(r.y + r.h) {
                    if ry >= height {
                        continue;
                    }
                    for rx in r.x..(r.x + r.w) {
                        if rx >= width {
                            continue;
                        }
                        out[ry as usize * width as usize + rx as usize] = r.id;
                    }
                }
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
    paths::asset_root().join("maps").join(format!("{}.map", id))
}

/// Load `assets/tile_palettes/{id}.json`. Invalid or missing file → `None`.
fn load_tile_palette(id: &str) -> Option<crate::map::TilePalette> {
    let path = crate::paths::asset_root()
        .join("tile_palettes")
        .join(format!("{id}.json"));
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
            Self::InvalidTiles(w, h, p) => {
                write!(f, "Invalid tiles ({}x{}) at {}", w, h, p.display())
            }
            Self::MissingPalette(id) => write!(f, "Missing tile palette: {}", id),
        }
    }
}

impl std::error::Error for MapLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e, _) => Some(e),
            Self::Json(e, _) => Some(e),
            Self::InvalidTiles(_, _, _) | Self::MissingPalette(_) => None,
        }
    }
}

/// Load map by id. Reads `map.json`, optional `npc.json`, optional `doors.json`, and optional `scenes.json` under `assets/maps/{id}.map/`.
pub fn load_map(id: &str) -> Result<MapData, MapLoadError> {
    let dir = map_dir(id);
    let map_path = dir.join("map.json");
    let npc_path = dir.join("npc.json");
    let props_path = dir.join("props.json");
    let doors_path = dir.join("doors.json");
    let scenes_path = dir.join("scenes.json");

    let map_data =
        std::fs::read_to_string(&map_path).map_err(|e| MapLoadError::Io(e, map_path.clone()))?;
    let m: MapJson =
        serde_json::from_str(&map_data).map_err(|e| MapLoadError::Json(e, map_path.clone()))?;

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

    let tileset = m
        .tileset
        .as_deref()
        .and_then(|id| crate::assets::load_tileset(id, m.tileset_tile_size));

    let npcs = match std::fs::read_to_string(&npc_path) {
        Ok(data) => {
            let entries: Vec<MapNpcEntry> =
                serde_json::from_str(&data).map_err(|e| MapLoadError::Json(e, npc_path.clone()))?;
            load_npcs_from_map(&entries)?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(MapLoadError::Io(e, npc_path.clone())),
    };

    let props = match std::fs::read_to_string(&props_path) {
        Ok(data) => serde_json::from_str::<Vec<MapPropEntry>>(&data)
            .map_err(|e| MapLoadError::Json(e, props_path.clone()))?,
        Err(_) => Vec::new(),
    };

    let doors = match std::fs::read_to_string(&doors_path) {
        Ok(data) => serde_json::from_str::<Vec<MapDoor>>(&data)
            .map_err(|e| MapLoadError::Json(e, doors_path.clone()))?,
        Err(_) => Vec::new(),
    };

    let scene_triggers = match std::fs::read_to_string(&scenes_path) {
        Ok(data) => serde_json::from_str::<Vec<MapSceneTrigger>>(&data)
            .map_err(|e| MapLoadError::Json(e, scenes_path.clone()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(MapLoadError::Io(e, scenes_path.clone())),
    };

    Ok(MapData {
        tilemap,
        npcs,
        props,
        doors,
        scene_triggers,
        player_start: m.player_start.unwrap_or(DEFAULT_PLAYER_START),
        tileset,
    })
}

/// Resolves each [`MapNpcEntry`] to a merged [`NpcConfig`]. Any broken character reference fails loudly.
fn load_npcs_from_map(entries: &[MapNpcEntry]) -> Result<Vec<(String, NpcConfig)>, MapLoadError> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let npc_dir = paths::asset_root().join("npc");
    let mut npcs = Vec::with_capacity(entries.len());
    for entry in entries {
        let char_cfg = load_character_npc(&npc_dir, &entry.id)?;
        let conversation_id = char_cfg.conversation_id.unwrap_or_else(|| entry.id.clone());
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
    Ok(npcs)
}

/// Load [`CharacterNpcConfig`] for `id`: `assets/npc/{id}.npc/config.json` first, else `assets/npc/{id}.npc.json` (legacy; logs deprecation).
///
/// Public for scene actors and tools that are not placed via map `npc.json`.
pub fn load_character_npc_config(id: &str) -> Result<CharacterNpcConfig, MapLoadError> {
    let npc_dir = crate::paths::asset_root().join("npc");
    load_character_npc(&npc_dir, id)
}

fn load_character_npc(
    npc_dir: &std::path::Path,
    id: &str,
) -> Result<CharacterNpcConfig, MapLoadError> {
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

/// Prints to stderr where this binary was built from and how the tileset decoded. If the path is
/// not the repo you are editing, you are running the wrong executable; if the grid is `1×1`,
/// every map cell will show the same sheet tile.
pub fn log_startup_tilemap_diagnostics(map_id: &str, data: &MapData) {
    let map_json = map_dir(map_id).join("map.json");
    eprintln!("[bangbang] Asset Root = {}", paths::asset_root().display());
    eprintln!(
        "[bangbang] map {} map.json present: {} ({})",
        map_id,
        map_json.display(),
        if map_json.exists() { "yes" } else { "NO" }
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
    use super::*;

    #[test]
    fn mumhome_second_floor_map_defines_tileset_draw_for_binary_grid() {
        let d = crate::map_loader::load_map("mumhome.secondFloor")
            .expect("mumhome.secondFloor map should load");
        let td = d.tilemap.tileset_draw.as_ref().expect(
            "mumhome.secondFloor.map must set tileset_draw: logical 0/1 are collision ids, not wang sheet indices",
        );
        assert!(td.wang_autotile);
        assert_eq!(td.floor, 6);
    }

    #[test]
    fn mumhome_second_floor_tileset_is_four_by_four_at_32px() {
        let d = crate::map_loader::load_map("mumhome.secondFloor")
            .expect("mumhome.secondFloor map should load");
        let s = d
            .tileset
            .expect("mumhome.secondFloor loads farwest_interior");
        assert_eq!((s.cols, s.rows, s.frame_width), (4, 4, 32));
    }

    #[test]
    fn mumhome_second_floor_has_no_npcs_when_npc_json_empty() {
        let d = crate::map_loader::load_map("mumhome.secondFloor").expect("map loads");
        assert!(d.npcs.is_empty());
    }

    #[test]
    fn mumhome_first_floor_loads_door_and_mom() {
        let d = crate::map_loader::load_map("mumhome.firstFloor").expect("map loads");
        assert_eq!(d.doors.len(), 2);
        let to_maps: Vec<&str> = d.doors.iter().map(|d| d.to_map.as_str()).collect();
        assert!(to_maps.contains(&"mumhome.secondFloor"));
        assert!(to_maps.contains(&"dustfall.junction"));
        assert_eq!(d.npcs.len(), 1);
        assert_eq!(d.npcs[0].0, "mom");
    }

    #[test]
    fn dustfall_junction_loads_farwest_ground_tileset() {
        let d = crate::map_loader::load_map("dustfall.junction").expect("map loads");
        let s = d
            .tileset
            .expect("dustfall.junction loads farwest_ground");
        assert_eq!((s.cols, s.rows, s.frame_width), (4, 4, 32));
        let td = d
            .tilemap
            .tileset_draw
            .as_ref()
            .expect("dustfall.junction sets tileset_draw for wang ground");
        assert!(td.wang_autotile);
        assert_eq!(td.floor, 6);
    }

    #[test]
    fn dustfall_junction_includes_billy_house_prop() {
        let d = crate::map_loader::load_map("dustfall.junction").expect("map loads");
        assert!(
            d.props.iter().any(|p| p.id == "billyHouse"),
            "props.json should list billyHouse for PixelLab house art"
        );
        for id in [
            "clinic",
            "sheriff",
            "bank",
            "saloon",
            "emporium",
            "hitchPost",
            "waterTrough",
            "barrels",
            "cactus",
        ] {
            assert!(
                d.props.iter().any(|p| p.id == id),
                "dustfall.junction props.json should list {id}"
            );
        }
        assert_eq!(
            d.props.iter().filter(|p| p.id == "cactus").count(),
            1,
            "dustfall.junction should place at least one cactus instance"
        );
    }

    #[test]
    fn load_npcs_from_map_errors_on_missing_character_config() {
        let entries = vec![MapNpcEntry {
            id: "__missing_character_config__".into(),
            position: [0.0, 0.0],
        }];
        assert!(
            load_npcs_from_map(&entries).is_err(),
            "broken npc references should fail instead of silently using fallback NPCs"
        );
    }

    #[test]
    fn sparse_tile_loading_generates_correct_grid() {
        let sparse = SparseTilesJson {
            fill: 0,
            perimeter: Some(1),
            rects: vec![SparseRect {
                id: 2,
                x: 1,
                y: 1,
                w: 2,
                h: 2,
            }],
        };
        let tiles = TilesJson::Sparse(sparse);
        let grid = tiles_row_major(4, 4, tiles).expect("sparse conversion works");
        // Row 0: Perimeter
        assert_eq!(&grid[0..4], &[1, 1, 1, 1]);
        // Row 1: Wall, Rect(2, 2), Wall
        assert_eq!(&grid[4..8], &[1, 2, 2, 1]);
        // Row 2: Wall, Rect(2, 2), Wall
        assert_eq!(&grid[8..12], &[1, 2, 2, 1]);
        // Row 3: Perimeter
        assert_eq!(&grid[12..16], &[1, 1, 1, 1]);
    }
}
