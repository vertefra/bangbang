//! Label: single-line text at (x, y).

use crate::software::draw_text;

/// Draw a single line of text at the given position.
pub fn draw_label(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
) {
    draw_text(buffer, width, height, x, y, text, color);
}
