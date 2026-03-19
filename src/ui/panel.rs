//! Panel: filled rectangle with optional top border.

use crate::software::fill_rect;

/// Draw a panel (fill rect + top border strip). Clips to buffer.
pub fn draw_panel(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    fill_color: u32,
    border_color: u32,
    border_top_px: i32,
) {
    fill_rect(buffer, width, height, left, top, right, bottom, fill_color);
    if border_top_px > 0 {
        fill_rect(
            buffer,
            width,
            height,
            left,
            top,
            right,
            top + border_top_px,
            border_color,
        );
    }
}
