//! Debug HUD: FPS and extra lines (Noto Sans Bold path in font atlas).

use crate::gpu::frame_context::DebugOverlay;
use crate::gpu::pass_common::SubBatch;
use crate::gpu::GpuRenderer;

pub(crate) fn draw_debug_pass(
    r: &mut GpuRenderer,
    overlay: Option<DebugOverlay>,
    ui_scale: u32,
    font_scale: f32,
    font: &mut SubBatch,
) {
    let Some(o) = overlay else {
        return;
    };
    // Linear black, alpha 1 — avoid `packed_rgb_to_linear(0)` which forces alpha 0.
    let fg = [0.0, 0.0, 0.0, 1.0];
    let ui_s = ui_scale.max(1) as f32;
    let text_s = ui_s * font_scale;
    let margin = 6.0 * text_s;
    let line_step = 12.0 * text_s;
    let mut y = margin;
    let fps_label = format!("FPS:{:.0}", o.fps);
    r.push_ui_debug_text(font, &fps_label, margin, y, fg, text_s, None);
    y += line_step;
    for line in &o.lines {
        r.push_ui_debug_text(font, line, margin, y, fg, text_s, None);
        y += line_step;
    }
}
