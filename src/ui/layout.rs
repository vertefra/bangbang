//! Layout: screen-space rects from theme and dimensions.

use super::theme::UiTheme;

/// Full-screen rect for the dialogue box at the bottom. Returns (left, top, right, bottom).
pub fn dialogue_box_rect(screen_w: u32, screen_h: u32, theme: &UiTheme) -> (i32, i32, i32, i32) {
    let h = screen_h as i32;
    let top = h - theme.dialogue_box_height;
    (0, top, screen_w as i32, h)
}

/// Text position for the dialogue message: (x, y) inside the dialogue box (with padding).
pub fn dialogue_text_pos(
    _screen_w: u32,
    _screen_h: u32,
    dialogue_top: i32,
    theme: &UiTheme,
) -> (i32, i32) {
    let x = theme.dialogue_padding_x;
    let y = dialogue_top + theme.dialogue_padding_y;
    (x, y)
}
