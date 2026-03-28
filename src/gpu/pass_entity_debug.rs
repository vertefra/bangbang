//! Per-sprite debug overlays when built with `--features debug` (red AABB border + position label).

use crate::assets::AssetStore;
use crate::ecs::{
    AnimationState, DoorMarker, Facing, MapProp, Sprite, SpriteSheet, Transform, World,
};
use crate::gpu::pass_common::{PassFrameParams, SubBatch};
use crate::gpu::pass_entities::compute_sprite_layout_and_payload;
use crate::gpu::GpuRenderer;
use crate::render::color::{packed_rgb_to_linear, to_u32};

/// One pass: pushes world position labels into `font`, returns red border quads for the encoder.
pub(crate) fn prepare_entity_debug_overlay(
    r: &mut GpuRenderer,
    world: &World,
    asset_store: &mut AssetStore,
    params: PassFrameParams,
    font: &mut SubBatch,
    ui_scale: u32,
    font_scale: f32,
) -> SubBatch {
    let fg = [0.0_f32, 0.0, 0.0, 1.0];
    let ui_s = ui_scale.max(1) as f32;
    let text_s = ui_s * font_scale;
    let line_h = 14.0 * text_s;
    let red = packed_rgb_to_linear(to_u32(1.0, 0.0, 0.0));
    let mut borders = SubBatch::default();

    for (_, (transform, sprite, sprite_sheet, facing, anim_state, map_prop, door_marker)) in world
        .query::<(
            &Transform,
            &Sprite,
            Option<&SpriteSheet>,
            Option<&Facing>,
            Option<&AnimationState>,
            Option<&MapProp>,
            Option<&DoorMarker>,
        )>()
        .iter()
    {
        let (layout, _) = compute_sprite_layout_and_payload(
            r,
            asset_store,
            transform,
            sprite,
            sprite_sheet,
            facing,
            anim_state,
            map_prop,
            door_marker,
            params,
        );
        let label = format!("{:.1},{:.1}", transform.position.x, transform.position.y);
        let y = layout.sy0 - line_h;
        r.push_ui_debug_text(font, &label, layout.sx0, y, fg, text_s, None);

        push_aabb_border_quads(
            &mut borders,
            layout.sx0,
            layout.sy0,
            layout.sx1,
            layout.sy1,
            1.0,
            red,
        );
    }

    borders
}

fn push_aabb_border_quads(
    batch: &mut SubBatch,
    sx0: f32,
    sy0: f32,
    sx1: f32,
    sy1: f32,
    t: f32,
    color: [f32; 4],
) {
    batch.push_quad(sx0, sy0, sx1, sy0 + t, 0.0, 0.0, 1.0, 1.0, color);
    batch.push_quad(sx0, sy1 - t, sx1, sy1, 0.0, 0.0, 1.0, 1.0, color);
    batch.push_quad(sx0, sy0 + t, sx0 + t, sy1 - t, 0.0, 0.0, 1.0, 1.0, color);
    batch.push_quad(sx1 - t, sy0 + t, sx1, sy1 - t, 0.0, 0.0, 1.0, 1.0, color);
}
