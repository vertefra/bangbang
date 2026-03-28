//! Layout: screen-space rects from theme and dimensions. Theme sizes are 1×; multiply by `ui_scale`.

use super::theme::UiTheme;

fn s(ui_scale: i32) -> i32 {
    ui_scale.max(1)
}

/// Full-screen rect for the dialogue box at the bottom. Returns (left, top, right, bottom).
pub fn dialogue_box_rect(
    screen_w: u32,
    screen_h: u32,
    theme: &UiTheme,
    ui_scale: i32,
) -> (i32, i32, i32, i32) {
    let sc = s(ui_scale);
    let h = screen_h as i32;
    let box_h = theme.dialogue_box_height * sc;
    let top = h - box_h;
    (0, top, screen_w as i32, h)
}

/// Extra horizontal offset for dialogue text when a portrait is shown (portrait width + gap), in screen px.
pub fn dialogue_portrait_text_extra_left(theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    theme.dialogue_portrait_width * sc + theme.dialogue_portrait_gap * sc
}

/// Portrait quad inside the dialogue box (left, aligned with text top padding). Screen px.
pub fn dialogue_portrait_rect(
    screen_w: u32,
    screen_h: u32,
    theme: &UiTheme,
    ui_scale: i32,
) -> (i32, i32, i32, i32) {
    let (_, dtop, _, _) = dialogue_box_rect(screen_w, screen_h, theme, ui_scale);
    let sc = s(ui_scale);
    let left = theme.dialogue_padding_x * sc;
    let top = dtop + theme.dialogue_padding_y * sc;
    let pw = theme.dialogue_portrait_width * sc;
    let ph = theme.dialogue_portrait_height * sc;
    (left, top, left + pw, top + ph)
}

/// Text position for the dialogue message: (x, y) inside the dialogue box (with padding).
/// `extra_left` is usually `0` or [`dialogue_portrait_text_extra_left`].
pub fn dialogue_text_pos(
    _screen_w: u32,
    _screen_h: u32,
    dialogue_top: i32,
    theme: &UiTheme,
    ui_scale: i32,
    extra_left: i32,
) -> (i32, i32) {
    let sc = s(ui_scale);
    let x = theme.dialogue_padding_x * sc + extra_left;
    let y = dialogue_top + theme.dialogue_padding_y * sc;
    (x, y)
}

/// Bottom band for transient overworld messages (blocked door, etc.). Full width; `(left, top, right, bottom)` in screen px.
pub fn overworld_toast_band_rect(
    screen_w: u32,
    screen_h: u32,
    theme: &UiTheme,
    ui_scale: i32,
) -> (i32, i32, i32, i32) {
    let sc = s(ui_scale);
    let h = screen_h as i32;
    let pad_y = theme.dialogue_padding_y * sc;
    let band_h = pad_y * 2 + 22 * sc;
    let margin_bot = 8 * sc;
    let bottom = h - margin_bot;
    let top = bottom - band_h;
    (0, top, screen_w as i32, bottom)
}

/// Text origin inside [`overworld_toast_band_rect`] (top-left of first line).
pub fn overworld_toast_text_pos(band_top: i32, theme: &UiTheme, ui_scale: i32) -> (i32, i32) {
    let sc = s(ui_scale);
    (
        theme.dialogue_padding_x * sc,
        band_top + theme.dialogue_padding_y * sc,
    )
}

/// Backpack panel rect centered on screen. Returns (left, top, right, bottom).
pub fn backpack_panel_rect(
    screen_w: u32,
    screen_h: u32,
    theme: &UiTheme,
    ui_scale: i32,
) -> (i32, i32, i32, i32) {
    let sc = s(ui_scale);
    let w = screen_w as i32;
    let h = screen_h as i32;
    let pw = theme.backpack_panel_width * sc;
    let ph = theme.backpack_panel_height * sc;
    let left = (w - pw) / 2;
    let top = (h - ph) / 2;
    (left, top, left + pw, top + ph)
}

/// Y position for the panel title "BACKPACK" at the very top of the panel content area.
pub fn backpack_panel_title_y(panel_top: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    panel_top + theme.backpack_padding * sc
}

fn backpack_panel_title_h_px(sc: i32) -> i32 {
    20 * sc
}

/// Y position for the hotkey hint line at the bottom of the panel.
pub fn backpack_hotkey_hint_y(panel_bottom: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    panel_bottom - theme.backpack_padding * sc - 14 * sc
}

/// Y position for the "Usable" section title (inside backpack panel, below panel title).
pub fn backpack_usable_title_y(panel_top: i32, theme: &UiTheme, ui_scale: i32) -> i32 {
    let sc = s(ui_scale);
    backpack_panel_title_y(panel_top, theme, ui_scale) + backpack_panel_title_h_px(sc) + 8 * sc
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
pub fn backpack_passive_slot_y(
    panel_top: i32,
    theme: &UiTheme,
    index: usize,
    ui_scale: i32,
) -> i32 {
    let sc = s(ui_scale);
    let py = backpack_passive_title_y(panel_top, theme, ui_scale);
    py + backpack_title_h_px(sc) + backpack_slot_stride_px(theme, sc) * index as i32
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

/// HP bar outer rect (left, top, right, bottom) in screen px from theme margins and bar size (× ui_scale).
pub fn hp_bar_outer_rect(theme: &UiTheme, ui_scale: i32) -> (i32, i32, i32, i32) {
    let sc = s(ui_scale);
    let l = theme.hp_bar_margin_x * sc;
    let t = theme.hp_bar_margin_y * sc;
    let r = l + theme.hp_bar_bar_width * sc;
    let b = t + theme.hp_bar_bar_height * sc;
    (l, t, r, b)
}

/// Inner rect for track and fill: inset from `outer` by optional border (base px × ui_scale).
pub fn hp_bar_inner_rect(
    theme: &UiTheme,
    ui_scale: i32,
    outer: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let (l, t, r, b) = outer;
    let bpx = theme.hp_bar_border_px.unwrap_or(0).saturating_mul(s(ui_scale));
    if bpx <= 0 {
        return outer;
    }
    let il = l.saturating_add(bpx);
    let it = t.saturating_add(bpx);
    let ir = r.saturating_sub(bpx);
    let ib = b.saturating_sub(bpx);
    if ir <= il || ib <= it {
        return outer;
    }
    (il, it, ir, ib)
}

/// Fill quad inside `inner`; `ratio` is clamped to [0, 1], width scales by ratio.
pub fn hp_bar_fill_rect(inner: (i32, i32, i32, i32), ratio: f32) -> (i32, i32, i32, i32) {
    let (il, it, ir, ib) = inner;
    let w = (ir - il).max(0);
    let rw = (w as f32 * ratio.clamp(0.0, 1.0)).round() as i32;
    let fr = (il + rw).min(ir);
    (il, it, fr, ib)
}

/// Label anchor (x, y) to the right of the bar; y is top padding inside outer bounds.
pub fn hp_bar_label_pos(outer: (i32, i32, i32, i32), ui_scale: i32) -> (i32, i32) {
    let (_, t, r, _) = outer;
    let gap = 6 * s(ui_scale);
    let pad = 2 * s(ui_scale);
    (r + gap, t + pad)
}
