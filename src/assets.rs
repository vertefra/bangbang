//! Load character sprite sheets from PNG (`assets/characters/{id}/sheet.png`, optional `sheet.json`)
//! and map tilesets from `assets/tiles/{id}.png` with optional `assets/tiles/{id}.json` (`tile_size`; square 4×4 sheets infer 32 or 16 from dimensions when JSON is missing).

use std::collections::HashMap;
use std::path::PathBuf;


fn assets_dir() -> PathBuf {
    crate::paths::asset_root()
}

fn character_dir(id: &str) -> PathBuf {
    assets_dir().join("characters").join(id)
}

#[derive(serde::Deserialize)]
struct SheetMeta {
    #[serde(default = "default_rows")]
    rows: u32,
    #[serde(default = "default_cols")]
    cols: u32,
}

fn default_rows() -> u32 {
    4
}
fn default_cols() -> u32 {
    2
}

/// Loaded sprite sheet: pixels 0x00RRGGBB, row-major; grid layout rows × cols.
#[derive(Clone, Debug)]
pub struct LoadedSheet {
    pub pixels: Vec<u32>,
    pub width: u32,
    pub height: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub rows: u32,
    pub cols: u32,
}

fn load_png(path: &std::path::Path) -> Option<(Vec<u32>, u32, u32)> {
    let img = image::open(path).ok()?.into_rgba8();
    let (w, h) = (img.width(), img.height());
    let pixels: Vec<u32> = img
        .pixels()
        .map(|p| {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            let a = p[3] as u32;
            if a == 0 {
                0
            } else {
                (r << 16) | (g << 8) | b
            }
        })
        .collect();
    Some((pixels, w, h))
}

/// If `{id}.json` is missing or `tile_size` does not divide the PNG, infer a square tile size so a
/// **4×4** sheet (16 Wang tiles) uses the intended frame — e.g. 128² → 32px, 64² → 16px. Using the
/// old default **16** on 128² wrongly yields an 8×8 grid; indices 0–15 then sample random 16×16
/// scraps of the real 32px tiles and the map looks like broken autotile soup.
fn infer_tile_size_for_square_sheet(width: u32, height: u32) -> u32 {
    if width == 0 || height == 0 || width != height {
        return 16;
    }
    for &ts in &[32u32, 16u32] {
        if width % ts != 0 {
            continue;
        }
        let n = width / ts;
        if n == 4 {
            return ts;
        }
    }
    16
}

/// Pick frame size for a map tileset PNG. `map_tile_size` (from `map.json`) wins over `{id}.json`.
/// If the chosen size makes a single cell the whole image on a large square sheet, infer 4×4 instead
/// — otherwise `software::draw` clamps every sheet index to `0`.
fn resolve_map_tile_size(
    width: u32,
    height: u32,
    map_tile_size: Option<u32>,
    json_tile_size: Option<u32>,
) -> u32 {
    let merged = map_tile_size.or(json_tile_size);
    let candidate = match merged {
        Some(ts) if ts > 0 && width % ts == 0 && height % ts == 0 => ts,
        _ => return infer_tile_size_for_square_sheet(width, height),
    };
    let cols = width / candidate;
    let rows = height / candidate;
    if cols == 1 && rows == 1 && width >= 48 && height >= 48 && width == height {
        infer_tile_size_for_square_sheet(width, height)
    } else {
        candidate
    }
}

/// Load `assets/tiles/{id}.png`. Per-frame size is `map_tile_size` from `map.json` if set, else
/// `assets/tiles/{id}.json` `tile_size`, else inferred for square 4×4 wang sheets.
pub fn load_tileset(id: &str, map_tile_size: Option<u32>) -> Option<LoadedSheet> {
    let tiles_dir = assets_dir().join("tiles");
    let png_path = tiles_dir.join(format!("{}.png", id));
    let (pixels, width, height) = load_png(&png_path)?;
    let json_ts = std::fs::read_to_string(tiles_dir.join(format!("{}.json", id)))
        .ok()
        .and_then(|s| serde_json::from_str::<TilesetMeta>(&s).ok())
        .map(|m| m.tile_size);
    let tile_size = resolve_map_tile_size(width, height, map_tile_size, json_ts);
    let cols = width / tile_size;
    let rows = height / tile_size;
    if cols == 0 || rows == 0 {
        return None;
    }
    Some(LoadedSheet {
        pixels,
        width,
        height,
        frame_width: tile_size,
        frame_height: tile_size,
        rows,
        cols,
    })
}

#[derive(serde::Deserialize)]
struct TilesetMeta {
    #[serde(default = "default_tile_size")]
    tile_size: u32,
}
fn default_tile_size() -> u32 {
    16
}

/// Load a character sheet by id. Tries `assets/characters/{id}/` then `assets/npc/{id}.npc/`. Returns None if sheet.png is missing.
pub fn load_character_sheet(id: &str) -> Option<LoadedSheet> {
    let dirs = [
        character_dir(id),
        assets_dir().join("npc").join(format!("{}.npc", id)),
    ];
    for dir in &dirs {
        let png_path = dir.join("sheet.png");
        if let Some((pixels, width, height)) = load_png(&png_path) {
            return load_sheet_from_dir(dir, pixels, width, height);
        }
    }
    None
}

fn load_sheet_from_dir(
    dir: &std::path::Path,
    pixels: Vec<u32>,
    width: u32,
    height: u32,
) -> Option<LoadedSheet> {

    let (rows, cols) = std::fs::read_to_string(dir.join("sheet.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<SheetMeta>(&s).ok())
        .map(|m| (m.rows, m.cols))
        .unwrap_or((4, 2));

    let frame_width = width / cols;
    let frame_height = height / rows;

    Some(LoadedSheet {
        pixels,
        width,
        height,
        frame_width,
        frame_height,
        rows,
        cols,
    })
}

/// Cache of loaded character sheets. Load on first use.
pub struct AssetStore {
    sheets: HashMap<String, LoadedSheet>,
}

impl AssetStore {
    pub fn new() -> Self {
        Self {
            sheets: HashMap::new(),
        }
    }

    /// Get sheet for character id; loads from disk on first request. Returns None if load fails.
    pub fn get_sheet(&mut self, id: &str) -> Option<&LoadedSheet> {
        if !self.sheets.contains_key(id) {
            if let Some(sheet) = load_character_sheet(id) {
                self.sheets.insert(id.to_string(), sheet);
            }
        }
        self.sheets.get(id)
    }
}

impl Default for AssetStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tileset_tests {
    use super::{infer_tile_size_for_square_sheet, load_tileset, resolve_map_tile_size};

    #[test]
    fn farwest_interior_is_four_by_four_at_32px() {
        let s = load_tileset("farwest_interior", None).expect("load farwest_interior");
        assert_eq!(
            (s.cols, s.rows, s.frame_width, s.frame_height),
            (4, 4, 32, 32),
            "128² wang sheets must use 32px frames; 16px default would be 8×8 and break indices"
        );
    }

    #[test]
    fn whole_image_tile_size_on_square_sheet_is_ignored() {
        assert_eq!(resolve_map_tile_size(128, 128, None, Some(128)), 32);
        assert_eq!(resolve_map_tile_size(128, 128, None, Some(32)), 32);
        assert_eq!(resolve_map_tile_size(128, 128, Some(32), None), 32);
        assert_eq!(infer_tile_size_for_square_sheet(128, 128), 32);
    }
}
