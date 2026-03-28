//! Load character sprite sheets from PNG (`assets/characters/{id}/sheet.png`, optional `sheet.json`),
//! NPC folders (`assets/npc/{id}.npc/sheet.png`), map props (`assets/props/{id}/sheet.png`),
//! and map tilesets from `assets/tiles/{id}.png` with optional `assets/tiles/{id}.json` (`tile_size`; square 4×4 sheets infer 32 or 16 from dimensions when JSON is missing).
//! Optional dialogue portraits: `assets/npc/{id}.npc/portrait.png` or `assets/characters/{id}/portrait.png`.

use std::collections::{HashMap, HashSet};
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
/// — otherwise every tileset index resolves to the first cell in the GPU tilemap pass.
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

fn props_dir(id: &str) -> PathBuf {
    assets_dir().join("props").join(id)
}

fn door_props_dir(id: &str) -> PathBuf {
    assets_dir().join("props").join(format!("{id}.door"))
}

fn map_props_dir(id: &str) -> PathBuf {
    assets_dir().join("props").join(format!("{id}.prop"))
}

fn snake_to_camel_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut uppercase_next = false;
    for ch in id.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            out.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn camel_to_snake_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len() + 4);
    for (i, ch) in id.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn push_unique(vec: &mut Vec<String>, value: String) {
    if !vec.iter().any(|existing| existing == &value) {
        vec.push(value);
    }
}

fn asset_id_variants(id: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return variants;
    }
    push_unique(&mut variants, trimmed.to_string());
    push_unique(&mut variants, snake_to_camel_id(trimmed));
    push_unique(&mut variants, camel_to_snake_id(trimmed));
    variants
}

fn first_existing_prop_folder(candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| props_dir(candidate).join("sheet.png").exists())
        .cloned()
}

/// Resolve a map `props.json` id to a folder under `assets/props/`.
///
/// Preferred convention is `assets/props/{id}.prop/` using camelCase ids like `billyHouse`.
/// Legacy plain folders and snake_case ids are still accepted during migration.
pub fn resolve_map_prop_sheet_id(id: &str) -> Option<String> {
    let variants = asset_id_variants(id);
    for variant in &variants {
        if map_props_dir(variant).join("sheet.png").exists() {
            return Some(format!("{variant}.prop"));
        }
    }
    first_existing_prop_folder(&variants)
}

/// Resolve a door `prop` id from `doors.json` to an actual folder under `assets/props/`.
///
/// Preferred convention is `assets/props/{id}.door/` using camelCase ids like `southHeavy`.
/// Legacy fallbacks keep older folders and snake_case ids working during migration.
pub fn resolve_door_prop_sheet_id(id: &str) -> Option<String> {
    let variants = asset_id_variants(id);
    for variant in &variants {
        if door_props_dir(variant).join("sheet.png").exists() {
            return Some(format!("{variant}.door"));
        }
    }
    let mut legacy_candidates = Vec::new();
    for variant in &variants {
        push_unique(&mut legacy_candidates, format!("door_{variant}"));
    }
    first_existing_prop_folder(&legacy_candidates)
}

    /// Load a character sheet by id. Tries `assets/characters/{id}/`, `assets/npc/{id}.npc/`,
    /// then `assets/props/{id}/` (including suffixed ids like `southHeavy.door` or `billyHouse.prop`).
    /// Returns None if sheet.png is missing.
pub fn load_character_sheet(id: &str) -> Option<LoadedSheet> {
    let dirs = [
        character_dir(id),
        assets_dir().join("npc").join(format!("{}.npc", id)),
        props_dir(id),
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

/// Cache key for a dedicated dialogue portrait in [`AssetStore::sheets`] and the GPU character texture map.
pub fn dialogue_portrait_asset_key(npc_id: &str) -> String {
    format!("{npc_id}__dialogue_portrait")
}

/// Cache key for a skill icon image (`assets/skills/{id}.skill_image.png`).
pub fn skill_image_key(skill_id: &str) -> String {
    format!("skill_icon:{skill_id}")
}

fn load_skill_image(skill_id: &str) -> Option<LoadedSheet> {
    let path = assets_dir()
        .join("skills")
        .join(format!("{skill_id}.skill_image.png"));
    let (pixels, width, height) = load_png(&path)?;
    if width == 0 || height == 0 {
        return None;
    }
    Some(LoadedSheet {
        pixels,
        width,
        height,
        frame_width: width,
        frame_height: height,
        rows: 1,
        cols: 1,
    })
}

/// Load optional dialogue portrait: tries `assets/npc/{id}.npc/portrait.png` then `assets/characters/{id}/portrait.png`.
/// Whole image is one frame (`rows`/`cols` = 1).
pub fn load_dialogue_portrait(id: &str) -> Option<LoadedSheet> {
    let paths = [
        assets_dir()
            .join("npc")
            .join(format!("{id}.npc"))
            .join("portrait.png"),
        character_dir(id).join("portrait.png"),
    ];
    for path in &paths {
        let (pixels, width, height) = load_png(path)?;
        if width == 0 || height == 0 {
            continue;
        }
        return Some(LoadedSheet {
            pixels,
            width,
            height,
            frame_width: width,
            frame_height: height,
            rows: 1,
            cols: 1,
        });
    }
    None
}

/// Cache of loaded character sheets. Load on first use.
pub struct AssetStore {
    sheets: HashMap<String, LoadedSheet>,
    /// NPC ids whose `portrait.png` was looked up and missing (avoid re-reading disk every frame).
    dialogue_portrait_misses: HashSet<String>,
}

impl AssetStore {
    pub fn new() -> Self {
        Self {
            sheets: HashMap::new(),
            dialogue_portrait_misses: HashSet::new(),
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

    /// Icon image for a skill, from `assets/skills/{id}.skill_image.png`. Cached; None if missing.
    pub fn get_skill_image(&mut self, skill_id: &str) -> Option<&LoadedSheet> {
        let key = skill_image_key(skill_id);
        if !self.sheets.contains_key(&key) {
            if let Some(sheet) = load_skill_image(skill_id) {
                self.sheets.insert(key.clone(), sheet);
            }
        }
        self.sheets.get(&key)
    }

    /// Dedicated dialogue portrait for `npc_id`, if present on disk. Cached; misses are remembered per id.
    pub fn get_dialogue_portrait_sheet(&mut self, npc_id: &str) -> Option<&LoadedSheet> {
        let key = dialogue_portrait_asset_key(npc_id);
        if self.dialogue_portrait_misses.contains(npc_id) {
            return None;
        }
        if !self.sheets.contains_key(&key) {
            match load_dialogue_portrait(npc_id) {
                Some(sheet) => {
                    self.sheets.insert(key.clone(), sheet);
                }
                None => {
                    self.dialogue_portrait_misses.insert(npc_id.to_string());
                }
            }
        }
        self.sheets.get(&key)
    }
}

impl Default for AssetStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tileset_tests {
    use super::{
        infer_tile_size_for_square_sheet, load_character_sheet, load_tileset,
        resolve_door_prop_sheet_id, resolve_map_prop_sheet_id, resolve_map_tile_size,
    };

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
    fn farwest_ground_is_four_by_four_at_32px() {
        let s = load_tileset("farwest_ground", None).expect("load farwest_ground");
        assert_eq!(
            (s.cols, s.rows, s.frame_width, s.frame_height),
            (4, 4, 32, 32),
            "dustfall.junction uses farwest_ground; grid must match wang LUT in render/mod.rs"
        );
    }

    #[test]
    fn whole_image_tile_size_on_square_sheet_is_ignored() {
        assert_eq!(resolve_map_tile_size(128, 128, None, Some(128)), 32);
        assert_eq!(resolve_map_tile_size(128, 128, None, Some(32)), 32);
        assert_eq!(resolve_map_tile_size(128, 128, Some(32), None), 32);
        assert_eq!(infer_tile_size_for_square_sheet(128, 128), 32);
    }

    #[test]
    fn south_heavy_alias_resolves_to_south_heavy_door_sheet() {
        let id = resolve_door_prop_sheet_id("southHeavy").expect("assets/props/southHeavy.door/");
        let s = load_character_sheet(&id).expect("load southHeavy door sheet");
        assert_eq!(
            (s.cols, s.rows, s.frame_width, s.frame_height),
            (1, 1, 64, 48)
        );
    }

    #[test]
    fn south_door_prop_sheet_loads() {
        let id = resolve_door_prop_sheet_id("south").expect("assets/props/south.door/");
        let s = load_character_sheet(&id).expect("load south door sheet");
        assert_eq!(
            (s.cols, s.rows, s.frame_width, s.frame_height),
            (1, 1, 48, 48)
        );
    }

    #[test]
    fn billy_house_alias_resolves_to_billy_house_prop_sheet() {
        let id = resolve_map_prop_sheet_id("billyHouse").expect("assets/props/billyHouse.prop/");
        let s = load_character_sheet(&id).expect("load billyHouse prop sheet");
        assert_eq!((s.cols, s.rows), (1, 1));
    }
}
