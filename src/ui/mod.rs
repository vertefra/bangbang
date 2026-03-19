//! UI layer: components (panel, label, bar), layout, theme. One draw entry per AppState.

mod bar;
mod label;
mod layout;
mod panel;
mod theme;

pub use theme::{load_theme, UiTheme};

use crate::software::to_u32;
use layout::{dialogue_box_rect, dialogue_text_pos};
use panel::draw_panel;
use label::draw_label;

/// Draw the dialogue box and message at the bottom. Used when AppState is Dialogue.
pub fn draw_dialogue(
    buffer: &mut [u32],
    w: u32,
    h: u32,
    theme: &UiTheme,
    message: &str,
) {
    let (left, top, right, bottom) = dialogue_box_rect(w, h, theme);
    let fill = to_u32(
        theme.dialogue_panel_fill[0],
        theme.dialogue_panel_fill[1],
        theme.dialogue_panel_fill[2],
    );
    let border = to_u32(
        theme.dialogue_panel_border[0],
        theme.dialogue_panel_border[1],
        theme.dialogue_panel_border[2],
    );
    let text_color = to_u32(
        theme.dialogue_text[0],
        theme.dialogue_text[1],
        theme.dialogue_text[2],
    );
    draw_panel(
        buffer,
        w,
        h,
        left,
        top,
        right,
        bottom,
        fill,
        border,
        theme.dialogue_border_top_px,
    );
    let (tx, ty) = dialogue_text_pos(w, h, top, theme);
    draw_label(buffer, w, h, tx, ty, message, text_color);
}

/// Draw overworld HUD (HP, money, etc.). No-op until Phase 6.
pub fn draw_overworld_hud(
    _buffer: &mut [u32],
    _w: u32,
    _h: u32,
    _theme: &UiTheme,
) {
}

/// Draw duel UI. Stub until Phase 2.
pub fn draw_duel(
    _buffer: &mut [u32],
    _w: u32,
    _h: u32,
    _theme: &UiTheme,
) {
}
