//! Layout: screen-space rects from theme and dimensions. Theme sizes are 1×; multiply by `ui_scale`.

use super::theme::UiTheme;

fn s(ui_scale: i32) -> i32 {
    ui_scale.max(1)
}

/// Full-screen rect for the dialogue box at the bottom. Returns (left, top, right, bottom).
pub fn dialogue_box_rect(screen_w: u32, screen_h: u32, theme: &UiTheme, ui_scale: i32) -> (i32, i32, i32, i32) {
    let sc = s(ui_scale);
    let h = screen_h as i32;
    let box_h = theme.dialogue_box_height * sc;
    let top = h - box_h;
    (0, top, screen_w as i32, h)
}

/// Text position for the dialogue message: (x, y) inside the dialogue box (with padding).
pub fn dialogue_text_pos(
    _screen_w: u32,
    _screen_h: u32,
    dialogue_top: i32,
    theme: &UiTheme,
    ui_scale: i32,
) -> (i32, i32) {
    let sc = s(ui_scale);
    let x = theme.dialogue_padding_x * sc;
    let y = dialogue_top + theme.dialogue_padding_y * sc;
    (x, y)
}

/// Backpack panel rect centered on screen. Returns (left, top, right, bottom).
pub fn backpack_panel_rect(screen_w: u32, screen_h: u32, theme: &UiTheme, ui_scale: i32) -> (i32, i32, i32, i32) {
    let sc = s(ui_scale);
    let w = screen_w as i32;
    let h = screen_h as i32;
    let pw = theme.backpack_panel_width * sc;
    let ph = theme.backpack_panel_height * sc;
    let left = (w - pw) / 2;
    let top = (h - ph) / 2;
    (left, top, left + pw, top + ph)
}

/// Y position for the "Usable Skills" section title (inside backpack panel).
pub fn backpack_usable_title_y(panel_top: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    panel_top + theme.backpack_padding * sc
}

/// Y position for the first usable slot (below usable title).
pub fn backpack_usable_slot_y(panel_top: i32, theme: &UiTheme, index: usize, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    let title_h = 20 * sc;
    let gap = 4 * sc;
    let slot_h = theme.backpack_slot_height * sc;
    panel_top + theme.backpack_padding * sc + title_h + (slot_h + gap) * index as i32
}

/// Max rows per backpack section (fixed layout reservation).
pub const BACKPACK_MAX_USABLE_SLOTS: usize = 8;
pub const BACKPACK_MAX_WEAPON_SLOTS: usize = 4;
pub const BACKPACK_MAX_PASSIVE_SLOTS: usize = 4;

fn backpack_title_h_px(sc: i32) -> i32 {
    20 * sc
}

fn backpack_slot_stride_px(theme: &UiTheme, sc: i32) -> i32 {
    theme.backpack_slot_height * sc + 4 * sc
}

fn backpack_section_gap_px(sc: i32) -> i32 {
    12 * sc
}

/// Y position for the "Weapons" section title (below the usable block).
pub fn backpack_weapon_title_y(panel_top: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    let th = backpack_title_h_px(sc);
    let stride = backpack_slot_stride_px(theme, sc);
    let usable_block = th + stride * BACKPACK_MAX_USABLE_SLOTS as i32;
    panel_top + theme.backpack_padding * sc + usable_block + backpack_section_gap_px(sc)
}

/// Y position for a weapon slot row.
pub fn backpack_weapon_slot_y(panel_top: i32, theme: &UiTheme, index: usize, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    let wy = backpack_weapon_title_y(panel_top, theme, ui_scale);
    wy + backpack_title_h_px(sc) + backpack_slot_stride_px(theme, sc) * index as i32
}

/// Y position for the "Passives" section title (below the weapons block).
pub fn backpack_passive_title_y(panel_top: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    let th = backpack_title_h_px(sc);
    let stride = backpack_slot_stride_px(theme, sc);
    let weapon_block = th + stride * BACKPACK_MAX_WEAPON_SLOTS as i32;
    backpack_weapon_title_y(panel_top, theme, ui_scale) + weapon_block + backpack_section_gap_px(sc)
}

/// Y position for a passive permanent slot row.
pub fn backpack_passive_slot_y(panel_top: i32, theme: &UiTheme, index: usize, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    let py = backpack_passive_title_y(panel_top, theme, ui_scale);
    py + backpack_title_h_px(sc) + backpack_slot_stride_px(theme, sc) * index as i32
}

/// Y position for the "Permanent Skills" section title.
pub fn backpack_permanent_title_y(panel_top: i32, theme: &UiTheme, usable_count: usize, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    let title_h = 20 * sc;
    let gap = 4 * sc;
    let slot_h = theme.backpack_slot_height * sc;
    let slot_area = if usable_count > 0 {
        title_h + (slot_h + gap) * usable_count as i32
    } else {
        0
    };
    panel_top + theme.backpack_padding * sc + title_h + slot_area + 12 * sc
}

/// Y position for a permanent slot.
pub fn backpack_permanent_slot_y(
    panel_top: i32,
    theme: &UiTheme,
    usable_count: usize,
    index: usize,
    ui_scale: i32,
) -> i32 {
    let sc = s(ui_scale);
    let section_y = backpack_permanent_title_y(panel_top, theme, usable_count, ui_scale);
    let title_h = 20 * sc;
    let gap = 4 * sc;
    let slot_h = theme.backpack_slot_height * sc;
    section_y + title_h + (slot_h + gap) * index as i32
}

/// X position for backpack content (title and slot labels).
pub fn backpack_content_x(panel_left: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    panel_left + theme.backpack_padding * sc
}

/// Horizontal indent for slot lines (after section titles).
pub fn backpack_slot_indent(ui_scale: i32) -> i32 {
    8 * s(ui_scale)
}
