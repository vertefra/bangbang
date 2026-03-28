//! Tilemap layer: solid underlay + textured tiles.

use crate::assets::LoadedSheet;
use crate::gpu::pass_common::{PassFrameParams, SubBatch};
use crate::gpu::GpuRenderer;
use crate::map::{Tilemap, LOGICAL_COBBLE_TILE_ID, LOGICAL_PATH_TILE_ID};
use crate::render::color::packed_rgb_to_linear;
use crate::render::{self, tilemap_is_binary_collision_only, wang_wall_sheet_index};

pub(crate) fn draw_tilemap_pass(
    r: &mut GpuRenderer,
    tilemap: &Tilemap,
    tileset: Option<&LoadedSheet>,
    params: PassFrameParams,
    white_under: &mut SubBatch,
    tiles: &mut SubBatch,
) {
    let PassFrameParams {
        cam_x,
        cam_y,
        half_w,
        half_h,
        rs,
    } = params;
    let world_to_screen_x = |wx: f32| -> f32 { (wx - cam_x) * rs + half_w };
    let world_to_screen_y = |wy: f32| -> f32 { (wy - cam_y) * rs + half_h };
    let ts = tilemap.tile_size;

    if let Some(sheet) = tileset {
        r.ensure_tileset(sheet);
        let tw = sheet.width as f32;
        let th = sheet.height as f32;

        for y in 0..tilemap.height {
            for x in 0..tilemap.width {
                let wx = x as f32 * ts;
                let wy = y as f32 * ts;
                let sx0 = world_to_screen_x(wx);
                let sy0 = world_to_screen_y(wy);
                let sx1 = world_to_screen_x(wx + ts);
                let sy1 = world_to_screen_y(wy + ts);
                if let Some(td) = &tilemap.tileset_draw {
                    let logical = tilemap.tile_at(x, y).unwrap_or(0);
                    let tile_id = if logical == 0 {
                        td.floor
                    } else if logical == LOGICAL_PATH_TILE_ID {
                        td.path.unwrap_or(td.floor)
                    } else if logical == LOGICAL_COBBLE_TILE_ID {
                        td.cobble.unwrap_or(td.floor)
                    } else if td.wang_autotile {
                        wang_wall_sheet_index(tilemap, x, y)
                    } else {
                        td.wall
                    };
                    let max_id = sheet.cols * sheet.rows;
                    let tid = tile_id.min(max_id.saturating_sub(1));
                    let col = tid % sheet.cols;
                    let row = tid / sheet.cols;
                    let src_x = col * sheet.frame_width;
                    let src_y = row * sheet.frame_height;
                    let u0 = src_x as f32 / tw;
                    let u1 = (src_x + sheet.frame_width) as f32 / tw;
                    let v0 = src_y as f32 / th;
                    let v1 = (src_y + sheet.frame_height) as f32 / th;
                    tiles.push_quad(sx0, sy0, sx1, sy1, u0, v0, u1, v1, [1.0, 1.0, 1.0, 1.0]);
                } else if tilemap_is_binary_collision_only(tilemap) {
                    let logical = tilemap.tile_at(x, y).unwrap_or(0);
                    let rgb = tilemap.fill_rgb_for_tile(logical);
                    let c = packed_rgb_to_linear(crate::render::to_u32(rgb[0], rgb[1], rgb[2]));
                    white_under.push_quad(sx0, sy0, sx1, sy1, 0.0, 0.0, 1.0, 1.0, c);
                } else {
                    let logical = tilemap.tile_at(x, y).unwrap_or(0);
                    let max_id = sheet.cols * sheet.rows;
                    let tid = logical.min(max_id.saturating_sub(1));
                    let col = tid % sheet.cols;
                    let row = tid / sheet.cols;
                    let src_x = col * sheet.frame_width;
                    let src_y = row * sheet.frame_height;
                    let u0 = src_x as f32 / tw;
                    let u1 = (src_x + sheet.frame_width) as f32 / tw;
                    let v0 = src_y as f32 / th;
                    let v1 = (src_y + sheet.frame_height) as f32 / th;
                    tiles.push_quad(sx0, sy0, sx1, sy1, u0, v0, u1, v1, [1.0, 1.0, 1.0, 1.0]);
                }
            }
        }
    } else {
        for y in 0..tilemap.height {
            for x in 0..tilemap.width {
                let wx = x as f32 * ts;
                let wy = y as f32 * ts;
                let sx0 = world_to_screen_x(wx);
                let sy0 = world_to_screen_y(wy);
                let sx1 = world_to_screen_x(wx + ts);
                let sy1 = world_to_screen_y(wy + ts);
                let logical = tilemap.tile_at(x, y).unwrap_or(0);
                let rgb = tilemap.fill_rgb_for_tile(logical);
                let c = packed_rgb_to_linear(render::to_u32(rgb[0], rgb[1], rgb[2]));
                white_under.push_quad(sx0, sy0, sx1, sy1, 0.0, 0.0, 1.0, 1.0, c);
            }
        }
    }
}
