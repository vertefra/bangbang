//! Screen-space UI: HP bar, dialogue, overworld toast, backpack (delegated).

use std::collections::BTreeMap;

use crate::assets::AssetStore;
use crate::gpu::pass_backpack::draw_backpack_pass;
use crate::gpu::pass_common::{theme_rgb, SubBatch};
use crate::gpu::GpuRenderer;
use crate::ui::{layout, UiTheme};

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_ui_pass(
    r: &mut GpuRenderer,
    theme: &UiTheme,
    player_hp: Option<(i32, i32)>,
    dialogue_message: Option<&str>,
    dialogue_text_extra_left: i32,
    overworld_toast: Option<&str>,
    backpack_open: bool,
    panel_lines: Option<&crate::ui::BackpackPanelLines>,
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

    if let Some((cur, max)) = player_hp {
        let ratio = cur as f32 / max as f32;
        let outer = layout::hp_bar_outer_rect(theme, us);
        let inner = layout::hp_bar_inner_rect(theme, us, outer);
        let (ol, ot, or, ob) = outer;
        let (il, it, ir, ib) = inner;
        let border_c = theme_rgb(theme.hp_bar_border);
        let track_c = theme_rgb(theme.hp_bar_track);
        let fill_c = theme_rgb(theme.hp_bar_fill);
        let label_c = theme_rgb(theme.hp_bar_label);

        if inner != outer {
            white_over.push_quad(
                ol as f32,
                ot as f32,
                or as f32,
                it as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                border_c,
            );
            white_over.push_quad(
                ol as f32,
                ib as f32,
                or as f32,
                ob as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                border_c,
            );
            white_over.push_quad(
                ol as f32,
                it as f32,
                il as f32,
                ib as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                border_c,
            );
            white_over.push_quad(
                ir as f32,
                it as f32,
                or as f32,
                ib as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                border_c,
            );
        }
        white_over.push_quad(
            il as f32,
            it as f32,
            ir as f32,
            ib as f32,
            0.0,
            0.0,
            1.0,
            1.0,
            track_c,
        );
        let (fl, ft, fr, fb) = layout::hp_bar_fill_rect(inner, ratio);
        if fr > fl {
            white_over.push_quad(
                fl as f32,
                ft as f32,
                fr as f32,
                fb as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                fill_c,
            );
        }
        let label = format!("{cur} / {max}");
        let (lx, ly) = layout::hp_bar_label_pos(outer, us);
        r.push_ui_text(
            font,
            &label,
            lx as f32,
            ly as f32,
            label_c,
            text_s,
            None,
        );
    }

    if let Some(msg) = dialogue_message {
        let (left, top, right, bottom) = layout::dialogue_box_rect(w, h, theme, us);
        let fill = theme_rgb(theme.dialogue_panel_fill);
        let border = theme_rgb(theme.dialogue_panel_border);
        let text_c = theme_rgb(theme.dialogue_text);
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
        let border_px = theme.dialogue_border_top_px * us;
        if border_px > 0 {
            white_over.push_quad(
                left as f32,
                top as f32,
                right as f32,
                (top + border_px) as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                border,
            );
        }
        let (tx, ty) =
            layout::dialogue_text_pos(w, h, top, theme, us, dialogue_text_extra_left);
        let inner_w =
            ((right - tx).saturating_sub(theme.dialogue_padding_x * us)).max(1) as f32;
        r.push_ui_text(
            font,
            msg,
            tx as f32,
            ty as f32,
            text_c,
            text_s,
            Some(inner_w),
        );
    }

    if backpack_open {
        if let Some(panel) = panel_lines {
            draw_backpack_pass(
                r,
                theme,
                panel,
                asset_store,
                w,
                h,
                ui_scale,
                font_scale,
                white_over,
                font,
                skill_icons,
            );
        }
    }

    if let Some(toast) = overworld_toast {
        let (left, top, right, bottom) =
            layout::overworld_toast_band_rect(w, h, theme, us);
        let fill = theme_rgb(theme.dialogue_panel_fill);
        let border = theme_rgb(theme.dialogue_panel_border);
        let text_c = theme_rgb(theme.dialogue_text);
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
        let border_px = theme.dialogue_border_top_px * us;
        if border_px > 0 {
            white_over.push_quad(
                left as f32,
                top as f32,
                right as f32,
                (top + border_px) as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                border,
            );
        }
        let (tx, ty) = layout::overworld_toast_text_pos(top, theme, us);
        let inner_w = ((right - tx).saturating_sub(theme.dialogue_padding_x * us)).max(1) as f32;
        r.push_ui_text(
            font,
            toast,
            tx as f32,
            ty as f32,
            text_c,
            text_s,
            Some(inner_w),
        );
    }
}
