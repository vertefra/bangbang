//! # CPU software renderer
//!
//! **High-level:** Renders the overworld (tilemap + entities) into a u32 framebuffer (0x00RRGGBB).
//! Entities with SpriteSheet draw from PNG; others use Sprite color. Camera follows the player.

use crate::assets::{AssetStore, LoadedSheet};
use crate::ecs::{AnimationKind, AnimationState, Direction, Facing, Player, Sprite, SpriteSheet, Transform, World};
use crate::map::Tilemap;
use std::num::NonZeroU32;

/// Convert normalized RGB (0.0–1.0) to u32 pixel 0x00RRGGBB.
///
/// **Rust:** `<< 16` / `<< 8` = bit shift; `|` = bitwise OR. We pack R in high byte, G middle, B low.
pub(crate) fn to_u32(r: f32, g: f32, b: f32) -> u32 {
    let r = (r.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (g.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (b.clamp(0.0, 1.0) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

fn direction_row(d: Direction) -> u32 {
    match d {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

/// Blit a region of a sprite sheet into the buffer. Source in sheet pixels; dest in screen pixels. Skips transparent (0).
fn blit_sheet(
    buffer: &mut [u32],
    buf_w: u32,
    buf_h: u32,
    sheet: &LoadedSheet,
    src_x: u32,
    src_y: u32,
    src_w: u32,
    src_h: u32,
    dest_left: i32,
    dest_top: i32,
    dest_w: i32,
    dest_h: i32,
) {
    if dest_w <= 0 || dest_h <= 0 {
        return;
    }
    for dy in 0..dest_h {
        for dx in 0..dest_w {
            let sx = src_x + (dx as u32 * src_w / dest_w as u32).min(src_w.saturating_sub(1));
            let sy = src_y + (dy as u32 * src_h / dest_h as u32).min(src_h.saturating_sub(1));
            let pixel = sheet.pixels[(sy * sheet.width + sx) as usize];
            if pixel == 0 {
                continue;
            }
            let bx = dest_left + dx;
            let by = dest_top + dy;
            if bx >= 0 && bx < buf_w as i32 && by >= 0 && by < buf_h as i32 {
                buffer[(by as u32 * buf_w + bx as u32) as usize] = pixel;
            }
        }
    }
}

/// Fill a rectangle in the buffer (screen coordinates). Clips to buffer bounds.
pub(crate) fn fill_rect(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: u32,
) {
    let left = left.clamp(0, width as i32);
    let right = right.clamp(0, width as i32);
    let top = top.clamp(0, height as i32);
    let bottom = bottom.clamp(0, height as i32);
    for y in top..bottom {
        for x in left..right {
            buffer[(y as u32 * width + x as u32) as usize] = color;
        }
    }
}

/// Minimal 5x7 bitmap font: one row per byte, low 5 bits used. Covers basic ASCII for dialogue.
fn glyph(c: u8) -> [u8; 7] {
    match c {
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        b'!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100],
        b'\'' => [0b00100, 0b00100, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        b'.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100],
        b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110],
        b'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        b'E' => [0b11111, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0b11111],
        b'F' => [0b11111, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0b10000],
        b'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        b'H' => [0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001, 0b10001],
        b'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'a' => [0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b10001, 0b01111],
        b'b' => [0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110],
        b'd' => [0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111],
        b'e' => [0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
        b'f' => [0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000],
        b'g' => [0b00000, 0b01111, 0b10001, 0b01111, 0b00001, 0b10001, 0b01110],
        b'h' => [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        b'i' => [0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'l' => [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'n' => [0b00000, 0b00000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        b'o' => [0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        b'r' => [0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        b's' => [0b00000, 0b00000, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        b't' => [0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110],
        b'u' => [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        b'v' => [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'w' => [0b00000, 0b00000, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010],
        _ => [0, 0, 0, 0, 0, 0, 0],
    }
}

const GLYPH_W: i32 = 5;
const GLYPH_STEP: i32 = 6;

pub(crate) fn draw_text(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    mut x: i32,
    y: i32,
    text: &str,
    color: u32,
) {
    for b in text.bytes() {
        let g = glyph(b);
        for (row, &bits) in g.iter().enumerate() {
            let sy = y + row as i32;
            if sy < 0 || sy >= height as i32 {
                continue;
            }
            for col in 0..GLYPH_W {
                if (bits >> (GLYPH_W - 1 - col)) & 1 != 0 {
                    let sx = x + col;
                    if sx >= 0 && sx < width as i32 {
                        buffer[(sy as u32 * width + sx as u32) as usize] = color;
                    }
                }
            }
        }
        x += GLYPH_STEP;
    }
}

/// Draw the overworld (tilemap + all sprites) into the buffer. Buffer is row-major, 0x00RRGGBB.
/// If `dialogue_message` is Some, draws a dialogue box via the UI module.
pub fn draw(
    buffer: &mut [u32],
    width: NonZeroU32,
    height: NonZeroU32,
    tilemap: &Tilemap,
    world: &World,
    dialogue_message: Option<&str>,
    asset_store: &mut AssetStore,
    theme: &crate::ui::UiTheme,
) {
    let w = width.get();
    let h = height.get();
    let cam_x = world
        .query::<(&Player, &Transform)>()
        .iter()
        .next()
        .map(|(_, (_, t))| t.position.x)
        .unwrap_or(0.0);
    let cam_y = world
        .query::<(&Player, &Transform)>()
        .iter()
        .next()
        .map(|(_, (_, t))| t.position.y)
        .unwrap_or(0.0);
    let half_w = w as f32 * 0.5;
    let half_h = h as f32 * 0.5;

    /// Scale world units to screen pixels. 2.0 = everything drawn 2× bigger.
    const RENDER_SCALE: f32 = 2.0;
    let world_to_screen_x = |wx: f32| ((wx - cam_x) * RENDER_SCALE + half_w) as i32;
    let world_to_screen_y = |wy: f32| ((wy - cam_y) * RENDER_SCALE + half_h) as i32;

    let bg = to_u32(0.15, 0.12, 0.18);
    for i in 0..buffer.len() {
        buffer[i] = bg;
    }

    let ts = tilemap.tile_size;
    let floor = to_u32(0.35, 0.38, 0.4);
    let wall = to_u32(0.2, 0.18, 0.22);

    for y in 0..tilemap.height {
        for x in 0..tilemap.width {
            let wx = x as f32 * ts;
            let wy = y as f32 * ts;
            let color = if tilemap.tile_at(x, y) == Some(0) {
                floor
            } else {
                wall
            };
            let sx = world_to_screen_x(wx);
            let sy = world_to_screen_y(wy);
            let sr = world_to_screen_x(wx + ts);
            let sb = world_to_screen_y(wy + ts);
            fill_rect(buffer, w, h, sx, sy, sr, sb, color);
        }
    }

    // Draw every entity with Transform + Sprite. If SpriteSheet is present and loaded, blit from sheet; else color rect.
    for (_, (transform, sprite, sprite_sheet, facing, anim_state)) in world
        .query::<(
            &Transform,
            &Sprite,
            Option<&SpriteSheet>,
            Option<&Facing>,
            Option<&AnimationState>,
        )>()
        .iter()
    {
        let row = facing.map(|f| direction_row(f.0)).unwrap_or(0);
        let (anim_kind, frame_index, walk_bob) = anim_state
            .map(|a| (a.kind, a.frame_index, matches!(a.kind, AnimationKind::Walk) && (a.frame_index % 2 == 1)))
            .unwrap_or((AnimationKind::Idle, 0, false));

        let (size_w, size_h, draw_sheet) = if let Some(ss) = sprite_sheet {
            asset_store.get_sheet(&ss.character_id).map(|sheet| {
            let cols = sheet.cols;
            let c = match anim_kind {
                AnimationKind::Idle => 0,
                AnimationKind::Walk => {
                    let walk_cols = (cols - 1).max(1);
                    (1 + (frame_index % walk_cols)).min(cols.saturating_sub(1))
                }
            };
            (
                sheet.frame_width as f32 * transform.scale.x,
                sheet.frame_height as f32 * transform.scale.y,
                Some((sheet, row.min(sheet.rows.saturating_sub(1)), c)),
            )
            })
        } else {
            None
        }.unwrap_or_else(|| {
            (
                16.0 * transform.scale.x,
                16.0 * transform.scale.y,
                None,
            )
        });

        let bob_off = if walk_bob { -1.0 } else { 0.0 };
        let wx = transform.position.x - size_w * 0.5;
        let wy = transform.position.y - size_h * 0.5 + bob_off;
        let sx = world_to_screen_x(wx);
        let sy = world_to_screen_y(wy);
        let sr = world_to_screen_x(wx + size_w);
        let sb = world_to_screen_y(wy + size_h);

        if let Some((sheet, row, col)) = draw_sheet {
            let src_x = col * sheet.frame_width;
            let src_y = row * sheet.frame_height;
            blit_sheet(
                buffer, w, h, sheet, src_x, src_y, sheet.frame_width, sheet.frame_height, sx, sy,
                sr - sx, sb - sy,
            );
        } else {
            let color = to_u32(sprite.color[0], sprite.color[1], sprite.color[2]);
            fill_rect(buffer, w, h, sx, sy, sr, sb, color);
        }
    }

    if let Some(msg) = dialogue_message {
        crate::ui::draw_dialogue(buffer, w, h, theme, msg);
    }
}
