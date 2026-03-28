//! Backpack panel: background, section headers, slot rows, skill icons, hotkey hint.

use std::collections::BTreeMap;

use crate::assets::AssetStore;
use crate::gpu::pass_common::{theme_rgb, SubBatch};
use crate::gpu::GpuRenderer;
use crate::ui::{layout, BackpackPanelLines, UiTheme};

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_backpack_pass(
    r: &mut GpuRenderer,
    theme: &UiTheme,
    panel: &BackpackPanelLines,
    asset_store: &mut AssetStore,
    w: u32,
    h: u32,
    ui_scale: u32,
    font_scale: f32,
    white_over: &mut SubBatch,
    font: &mut SubBatch,
    skill_icons: &mut BTreeMap<String, SubBatch>,
) {
    let ui_s = ui_scale.max(1) as f32;
    let text_s = ui_s * font_scale;
    let us = ui_scale.max(1) as i32;

    let (left, top, right, bottom) = layout::backpack_panel_rect(w, h, theme, us);
    let fill = theme_rgb(theme.backpack_panel_fill);
    let border = theme_rgb(theme.backpack_panel_border);
    let empty_c = theme_rgb(theme.backpack_slot_empty);
    let title_c = theme_rgb(theme.backpack_panel_title);
    let section_usable_c = theme_rgb(theme.backpack_section_usable);
    let section_weapon_c = theme_rgb(theme.backpack_section_weapon);
    let section_passive_c = theme_rgb(theme.backpack_section_passive);
    let row_weapon_c = theme_rgb(theme.backpack_row_weapon);
    let row_passive_c = theme_rgb(theme.backpack_row_passive);
    let row_equipped_c = theme_rgb(theme.backpack_row_equipped);

    white_over.push_quad(
        left as f32,
        top as f32,
        right as f32,
        bottom as f32,
        0.0,
        0.0,
        1.0,
        1.0,
        fill,
    );
    let bt = theme.backpack_border_top_px * us;
    if bt > 0 {
        white_over.push_quad(
            left as f32,
            top as f32,
            right as f32,
            (top + bt) as f32,
            0.0,
            0.0,
            1.0,
            1.0,
            border,
        );
    }

    let cx = layout::backpack_content_x(left, theme, us) as f32;
    let panel_inner_w =
        ((right - left).saturating_sub(theme.backpack_padding * us * 2)).max(1) as f32;

    // Slot icon size equals slot height; text starts after icon + small gap.
    let icon_sz = (theme.backpack_slot_height * us) as f32;
    let icon_gap = 4.0 * ui_s;
    let slot_text_x = cx + icon_sz + icon_gap;
    let slot_text_max_w = (panel_inner_w - icon_sz - icon_gap).max(8.0);

    // Panel title
    let pt_y = layout::backpack_panel_title_y(top, theme, us) as f32;
    r.push_ui_text(
        font,
        "BACKPACK",
        cx,
        pt_y,
        title_c,
        text_s * 1.1,
        Some(panel_inner_w),
    );

    // --- Usable ---
    let u_ty = layout::backpack_usable_title_y(top, theme, us) as f32;
    r.push_ui_text(
        font,
        "Usable  [2]",
        cx,
        u_ty,
        section_usable_c,
        text_s,
        Some(panel_inner_w),
    );
    let max_usable = layout::BACKPACK_MAX_USABLE_SLOTS;
    let usable_count = panel.usable.len().min(max_usable);
    for i in 0..max_usable {
        let slot_y = layout::backpack_usable_slot_y(top, theme, i, us) as f32;
        if let Some(slot) = panel.usable.get(i) {
            r.push_skill_icon(
                asset_store,
                &slot.skill_id,
                cx,
                slot_y,
                icon_sz,
                skill_icons,
            );
            r.push_ui_text(
                font,
                &slot.label,
                slot_text_x,
                slot_y,
                section_usable_c,
                text_s,
                Some(slot_text_max_w),
            );
        } else if i < max_usable {
            r.push_ui_text(
                font,
                "—",
                slot_text_x,
                slot_y,
                empty_c,
                text_s,
                Some(slot_text_max_w),
            );
        }
        let _ = usable_count;
    }

    // --- Weapons ---
    let w_ty = layout::backpack_weapon_title_y(top, theme, us) as f32;
    r.push_ui_text(
        font,
        "Weapons  [Tab]",
        cx,
        w_ty,
        section_weapon_c,
        text_s,
        Some(panel_inner_w),
    );
    let max_weapon = layout::BACKPACK_MAX_WEAPON_SLOTS;
    let weapon_count = panel.weapons.len().min(max_weapon);
    for i in 0..max_weapon {
        let slot_y = layout::backpack_weapon_slot_y(top, theme, i, us) as f32;
        if let Some(slot) = panel.weapons.get(i) {
            let c = if slot.is_equipped {
                row_equipped_c
            } else {
                row_weapon_c
            };
            r.push_skill_icon(
                asset_store,
                &slot.skill_id,
                cx,
                slot_y,
                icon_sz,
                skill_icons,
            );
            r.push_ui_text(
                font,
                &slot.label,
                slot_text_x,
                slot_y,
                c,
                text_s,
                Some(slot_text_max_w),
            );
        } else if i < max_weapon {
            r.push_ui_text(
                font,
                "—",
                slot_text_x,
                slot_y,
                empty_c,
                text_s,
                Some(slot_text_max_w),
            );
        }
        let _ = weapon_count;
    }

    // --- Passives ---
    let p_ty = layout::backpack_passive_title_y(top, theme, us) as f32;
    r.push_ui_text(
        font,
        "Passives",
        cx,
        p_ty,
        section_passive_c,
        text_s,
        Some(panel_inner_w),
    );
    let max_passive = layout::BACKPACK_MAX_PASSIVE_SLOTS;
    let passive_count = panel.passives.len().min(max_passive);
    for i in 0..max_passive {
        let slot_y = layout::backpack_passive_slot_y(top, theme, i, us) as f32;
        if let Some(slot) = panel.passives.get(i) {
            r.push_skill_icon(
                asset_store,
                &slot.skill_id,
                cx,
                slot_y,
                icon_sz,
                skill_icons,
            );
            r.push_ui_text(
                font,
                &slot.label,
                slot_text_x,
                slot_y,
                row_passive_c,
                text_s,
                Some(slot_text_max_w),
            );
        } else if i < max_passive {
            r.push_ui_text(
                font,
                "—",
                slot_text_x,
                slot_y,
                empty_c,
                text_s,
                Some(slot_text_max_w),
            );
        }
        let _ = passive_count;
    }

    // Hotkey hint at bottom
    let hint_y = layout::backpack_hotkey_hint_y(bottom, theme, us) as f32;
    r.push_ui_text(
        font,
        "[B] Close  [1] Fire  [2] Use",
        cx,
        hint_y,
        empty_c,
        text_s * 0.85,
        Some(panel_inner_w),
    );
}
