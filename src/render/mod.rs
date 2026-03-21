use crate::map::Tilemap;
use crate::ecs::Direction;

pub mod color;
pub mod text;

pub use color::*;
pub use text::*;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RenderScale(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UiScale(pub u32);

pub fn tilemap_is_binary_collision_only(tilemap: &Tilemap) -> bool {
    tilemap.tiles.iter().all(|&t| t <= 1)
}

pub fn facing_sprite_row(d: Direction) -> u32 {
    match d {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

const WANG16_SHEET_LUT: [u32; 16] = [
    4, 15, 1, 14, 10, 8, 2, 12, 11, 13, 7, 5, 9, 6, 0, 3,
];

fn tile_blocking_oob_wall(tilemap: &Tilemap, xi: i32, yi: i32) -> bool {
    if xi < 0 || yi < 0 {
        return true;
    }
    let x = xi as u32;
    let y = yi as u32;
    if x >= tilemap.width || y >= tilemap.height {
        return true;
    }
    tilemap.tile_at(x, y).map(|t| t != 0).unwrap_or(true)
}

fn wang_corner_signature_and(tilemap: &Tilemap, x: u32, y: u32) -> usize {
    let xi = x as i32;
    let yi = y as i32;
    let n = tile_blocking_oob_wall(tilemap, xi, yi - 1);
    let e = tile_blocking_oob_wall(tilemap, xi + 1, yi);
    let s = tile_blocking_oob_wall(tilemap, xi, yi + 1);
    let w = tile_blocking_oob_wall(tilemap, xi - 1, yi);
    let nw = usize::from(n && w);
    let ne = usize::from(n && e);
    let sw = usize::from(s && w);
    let se = usize::from(s && e);
    nw * 8 + ne * 4 + sw * 2 + se
}

fn wang_corner_signature_three(tilemap: &Tilemap, x: u32, y: u32) -> usize {
    let xi = x as i32;
    let yi = y as i32;
    let nw = tile_blocking_oob_wall(tilemap, xi - 1, yi - 1)
        || tile_blocking_oob_wall(tilemap, xi, yi - 1)
        || tile_blocking_oob_wall(tilemap, xi - 1, yi);
    let ne = tile_blocking_oob_wall(tilemap, xi, yi - 1)
        || tile_blocking_oob_wall(tilemap, xi + 1, yi - 1)
        || tile_blocking_oob_wall(tilemap, xi + 1, yi);
    let sw = tile_blocking_oob_wall(tilemap, xi - 1, yi)
        || tile_blocking_oob_wall(tilemap, xi - 1, yi + 1)
        || tile_blocking_oob_wall(tilemap, xi, yi + 1);
    let se = tile_blocking_oob_wall(tilemap, xi + 1, yi)
        || tile_blocking_oob_wall(tilemap, xi, yi + 1)
        || tile_blocking_oob_wall(tilemap, xi + 1, yi + 1);
    let nw = usize::from(nw);
    let ne = usize::from(ne);
    let sw = usize::from(sw);
    let se = usize::from(se);
    nw * 8 + ne * 4 + sw * 2 + se
}

pub fn wang_wall_sheet_index(tilemap: &Tilemap, x: u32, y: u32) -> u32 {
    let and_sig = wang_corner_signature_and(tilemap, x, y);
    let sig = if and_sig == 0 {
        wang_corner_signature_three(tilemap, x, y)
    } else {
        and_sig
    };
    WANG16_SHEET_LUT[sig]
}
