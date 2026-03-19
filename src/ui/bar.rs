//! Bar: horizontal fill (e.g. HP) with background and filled portion.

use crate::software::fill_rect;

/// Draw a bar: background rect then filled portion from left by ratio in [0.0, 1.0].
#[allow(dead_code)] // Used in Phase 6 (HUD) and Duel UI
pub fn draw_bar(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    ratio_0_1: f32,
    fill_color: u32,
    empty_color: u32,
) {
    fill_rect(buffer, width, height, left, top, right, bottom, empty_color);
    let r = ratio_0_1.clamp(0.0, 1.0);
    if r > 0.0 {
        let fill_w = ((right - left) as f32 * r) as i32;
        if fill_w > 0 {
            fill_rect(buffer, width, height, left, top, left + fill_w, bottom, fill_color);
        }
    }
}
