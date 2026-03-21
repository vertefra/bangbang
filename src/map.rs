//! # Tilemap module
//!
//! **High-level:** Represents a 2D grid of logical tile ids. Collision and solid-color appearance come
//! only from `tile_palette` (`walkable` + `color` per id). Unknown ids block movement and draw as
//! magenta in fill mode. Maps are loaded via `map_loader`. Optional `tileset_draw` maps logical cells
//! to sheet indices when rendering a tileset (art only; collision stays palette-driven).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RGB in 0.0–1.0 for `TilePalette::loader_fallback` (bordered demo map when load fails).
const FALLBACK_FLOOR_RGB: [f32; 3] = [0.35, 0.38, 0.4];
const FALLBACK_WALL_RGB: [f32; 3] = [0.2, 0.18, 0.22];

/// Solid fill when no tile id matches the palette (data error).
pub const MISSING_TILE_RGB: [f32; 3] = [1.0, 0.0, 1.0];

/// One logical tile id: fill color without a tileset, and whether the player can stand on it.
#[derive(Clone, Debug, Deserialize)]
pub struct TilePaletteEntry {
    pub color: [f32; 3],
    pub walkable: bool,
}

/// Colors and walkability per tile id (from `assets/tile_palettes/{id}.json`).
#[derive(Clone, Debug, Default)]
pub struct TilePalette {
    pub tiles: HashMap<u32, TilePaletteEntry>,
}

impl TilePalette {
    /// Palette used by `map_loader::fallback_tilemap` (ids 0 / 1 only).
    pub fn loader_fallback() -> Self {
        let mut tiles = HashMap::new();
        tiles.insert(
            0,
            TilePaletteEntry {
                color: FALLBACK_FLOOR_RGB,
                walkable: true,
            },
        );
        tiles.insert(
            1,
            TilePaletteEntry {
                color: FALLBACK_WALL_RGB,
                walkable: false,
            },
        );
        Self { tiles }
    }

    /// Parse JSON object `{ "0": { "color": [r,g,b], "walkable": true }, ... }` (keys = tile ids).
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        let raw: HashMap<String, TilePaletteEntry> = serde_json::from_str(s)?;
        let mut tiles = HashMap::with_capacity(raw.len());
        for (k, v) in raw {
            if let Ok(id) = k.parse::<u32>() {
                tiles.insert(id, v);
            }
        }
        Ok(TilePalette { tiles })
    }
}

/// When set, rendering maps each cell’s **logical** tile id to sheet indices (see `software::draw`).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TilesetDraw {
    pub floor: u32,
    pub wall: u32,
    /// If true, each blocking cell picks a Wang-style tile from the 4×4 sheet from neighbor layout
    /// (CPU renderer). `wall` is ignored for drawing in that case.
    #[serde(default)]
    pub wang_autotile: bool,
}

/// Tilemap data: grid of tile IDs stored row-major (`index = y * width + x`).
///
/// **Rust:** `Vec<u32>` is a growable heap-allocated array.
#[derive(Clone, Debug)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u32>,
    pub tile_size: f32,
    pub tileset_draw: Option<TilesetDraw>,
    pub tile_palette: TilePalette,
}

impl Tilemap {
    /// Linear index into `tiles` for (x, y), or `None` if out of bounds. **Rust:** `Option<usize>` = `Some(i)` or `None`.
    pub fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.width && y < self.height {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }

    /// Tile ID at (x, y), or `None` if out of bounds.
    pub fn tile_at(&self, x: u32, y: u32) -> Option<u32> {
        self.index(x, y).map(|i| self.tiles[i])
    }

    /// Blocking if out of bounds, if the tile id is missing from the palette, or if `walkable` is false.
    pub fn is_blocking(&self, x: u32, y: u32) -> bool {
        let Some(id) = self.tile_at(x, y) else {
            return true;
        };
        self.tile_palette
            .tiles
            .get(&id)
            .map(|e| !e.walkable)
            .unwrap_or(true)
    }

    /// RGB (0–1) for solid-color tile drawing when no tileset is loaded. Unknown id → [`MISSING_TILE_RGB`].
    pub fn fill_rgb_for_tile(&self, id: u32) -> [f32; 3] {
        self.tile_palette
            .tiles
            .get(&id)
            .map(|e| e.color)
            .unwrap_or(MISSING_TILE_RGB)
    }

    /// Total width in world units (pixels).
    pub fn width_pixels(&self) -> f32 {
        self.width as f32 * self.tile_size
    }

    /// Total height in world units (pixels).
    pub fn height_pixels(&self) -> f32 {
        self.height as f32 * self.tile_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tiny_map(tiles: Vec<u32>, palette: TilePalette) -> Tilemap {
        let w = 3u32;
        Tilemap {
            width: w,
            height: 1,
            tiles,
            tile_size: 32.0,
            tileset_draw: None,
            tile_palette: palette,
        }
    }

    fn palette_0_1() -> TilePalette {
        let mut tiles_map = HashMap::new();
        tiles_map.insert(
            0,
            TilePaletteEntry {
                color: [0.35, 0.38, 0.4],
                walkable: true,
            },
        );
        tiles_map.insert(
            1,
            TilePaletteEntry {
                color: [0.2, 0.18, 0.22],
                walkable: false,
            },
        );
        TilePalette { tiles: tiles_map }
    }

    #[test]
    fn is_blocking_from_palette_only() {
        let m = tiny_map(vec![0, 1, 2], palette_0_1());
        assert!(!m.is_blocking(0, 0));
        assert!(m.is_blocking(1, 0));
        assert!(m.is_blocking(2, 0));
    }

    #[test]
    fn is_blocking_respects_walkable_flag() {
        let mut tiles_map = HashMap::new();
        tiles_map.insert(
            2,
            TilePaletteEntry {
                color: [0.5, 0.5, 0.5],
                walkable: true,
            },
        );
        let m = tiny_map(vec![2], TilePalette { tiles: tiles_map });
        assert!(!m.is_blocking(0, 0));
    }

    #[test]
    fn fill_rgb_uses_palette() {
        let mut tiles_map = HashMap::new();
        tiles_map.insert(
            0,
            TilePaletteEntry {
                color: [0.1, 0.2, 0.3],
                walkable: true,
            },
        );
        let m = tiny_map(vec![0], TilePalette { tiles: tiles_map });
        assert_eq!(m.fill_rgb_for_tile(0), [0.1, 0.2, 0.3]);
    }

    #[test]
    fn fill_rgb_unknown_tile_is_magenta() {
        let m = tiny_map(vec![99], palette_0_1());
        assert_eq!(m.fill_rgb_for_tile(99), MISSING_TILE_RGB);
    }

    #[test]
    fn tile_palette_from_json_str() {
        let j = r#"{"0":{"color":[1.0,0.0,0.0],"walkable":true}}"#;
        let p = TilePalette::from_json_str(j).unwrap();
        assert_eq!(p.tiles.get(&0).unwrap().color, [1.0, 0.0, 0.0]);
        assert!(p.tiles.get(&0).unwrap().walkable);
    }

    #[test]
    fn tile_palette_json_requires_walkable() {
        let j = r#"{"0":{"color":[1.0,0.0,0.0]}}"#;
        assert!(TilePalette::from_json_str(j).is_err());
    }
}
