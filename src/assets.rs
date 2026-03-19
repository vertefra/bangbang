//! Load character sprite sheets from PNG. Path: `assets/characters/{id}/sheet.png` and optional `sheet.json`.

use std::collections::HashMap;
use std::path::PathBuf;

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
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
