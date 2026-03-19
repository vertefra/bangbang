//! # Tilemap module
//!
//! **High-level:** Represents a 2D grid of tiles for the overworld. Tile ID 0 = floor (walkable),
//! non-zero = wall (blocking). Provides indexing and collision checks. Maps are loaded via map_loader.

/// Tilemap data: grid of tile IDs. Tile 0 = walkable, non-zero = blocking.
///
/// **Rust:** `Vec<u32>` is a growable heap-allocated array. Tiles stored row-major: index = y*width+x.
#[derive(Clone, Debug)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u32>,
    pub tile_size: f32,
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

    /// Tile ID 0 = walkable; any other = blocking. Out-of-bounds = blocking. **Rust:** `.unwrap_or(true)` = use `true` when `None`.
    pub fn is_blocking(&self, x: u32, y: u32) -> bool {
        self.tile_at(x, y).map(|id| id != 0).unwrap_or(true)
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
