//! World sprites and dialogue portrait batching (Y-sorted chunks, texture batching).

use crate::assets::{dialogue_portrait_asset_key, AssetStore};
use crate::ecs::{
    AnimationKind, AnimationState, Direction, DoorMarker, Facing, MapProp, Sprite, SpriteSheet,
    Transform, World,
};
use crate::gpu::pass_common::{PassFrameParams, SubBatch};
use crate::gpu::GpuRenderer;
use crate::render::color::sprite_color_to_linear;
use crate::render::{self, facing_sprite_row};
use crate::ui::{layout, UiTheme};

/// World sprites in back-to-front order (smaller world depth Y drawn first).
pub(crate) enum EntityDrawChunk {
    Textured {
        character_id: String,
        batch: SubBatch,
    },
    Solid {
        batch: SubBatch,
    },
}

pub(crate) fn draw_entities_pass(
    r: &mut GpuRenderer,
    world: &World,
    asset_store: &mut AssetStore,
    params: PassFrameParams,
) -> Vec<EntityDrawChunk> {
    let PassFrameParams {
        cam_x,
        cam_y,
        half_w,
        half_h,
        rs,
    } = params;
    let world_to_screen_x = |wx: f32| -> f32 { (wx - cam_x) * rs + half_w };
    let world_to_screen_y = |wy: f32| -> f32 { (wy - cam_y) * rs + half_h };

    enum PendingDraw {
        Tex {
            cid: String,
            sx0: f32,
            sy0: f32,
            sx1: f32,
            sy1: f32,
            u0: f32,
            v0: f32,
            u1: f32,
            v1: f32,
        },
        Solid {
            sx0: f32,
            sy0: f32,
            sx1: f32,
            sy1: f32,
            c: [f32; 4],
        },
    }

    let mut scored: Vec<(f32, u32, PendingDraw)> = Vec::new();
    let mut seq: u32 = 0;

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
        let row = facing.map(|f| facing_sprite_row(f.0)).unwrap_or(0);
        let (anim_kind, frame_index, walk_bob) = anim_state
            .map(|a| {
                (
                    a.kind,
                    a.frame_index,
                    matches!(a.kind, AnimationKind::Walk) && (a.frame_index % 2 == 1),
                )
            })
            .unwrap_or((AnimationKind::Idle, 0, false));

        let (size_w, size_h, char_draw) = if let Some(ss) = sprite_sheet {
            if let Some(sheet) = asset_store.get_sheet(&ss.character_id) {
                r.ensure_character(&ss.character_id, sheet);
                let cols = sheet.cols;
                let c = match anim_kind {
                    AnimationKind::Idle => 0,
                    AnimationKind::Walk => {
                        let walk_cols = (cols - 1).max(1);
                        (1 + (frame_index % walk_cols)).min(cols.saturating_sub(1))
                    }
                };
                (
                    sheet.frame_width as f32 * transform.scale.x,
                    sheet.frame_height as f32 * transform.scale.y,
                    Some((
                        ss.character_id.clone(),
                        sheet,
                        row.min(sheet.rows.saturating_sub(1)),
                        c,
                    )),
                )
            } else {
                (16.0 * transform.scale.x, 16.0 * transform.scale.y, None)
            }
        } else {
            (16.0 * transform.scale.x, 16.0 * transform.scale.y, None)
        };

        let bob_off = if walk_bob { -1.0 } else { 0.0 };
        let wx = transform.position.x - size_w * 0.5;
        let wy = transform.position.y - size_h * 0.5 + bob_off;
        let sx0 = world_to_screen_x(wx);
        let sy0 = world_to_screen_y(wy);
        let sx1 = world_to_screen_x(wx + size_w);
        let sy1 = world_to_screen_y(wy + size_h);

        // Tall map props / doors: sort by sprite center so actors south of the building
        // (larger foot Y) draw in front of the foundation. Actors use foot (bottom edge) depth.
        let static_building = map_prop.is_some() || door_marker.is_some();
        let depth_y = if static_building {
            transform.position.y
        } else {
            transform.position.y + size_h * 0.5 + bob_off
        };

        let pending = if let Some((cid, sheet, r_row, col)) = char_draw {
            let src_x = col * sheet.frame_width;
            let src_y = r_row * sheet.frame_height;
            let tw = sheet.width as f32;
            let th = sheet.height as f32;
            let u0 = src_x as f32 / tw;
            let u1 = (src_x + sheet.frame_width) as f32 / tw;
            let v0 = src_y as f32 / th;
            let v1 = (src_y + sheet.frame_height) as f32 / th;
            PendingDraw::Tex {
                cid,
                sx0,
                sy0,
                sx1,
                sy1,
                u0,
                v0,
                u1,
                v1,
            }
        } else {
            let c = sprite_color_to_linear(sprite.color);
            PendingDraw::Solid {
                sx0,
                sy0,
                sx1,
                sy1,
                c,
            }
        };

        seq = seq.wrapping_add(1);
        scored.push((depth_y, seq, pending));
    }

    scored.sort_by(|(da, sa, _), (db, sb, _)| da.total_cmp(db).then_with(|| sa.cmp(sb)));

    let mut chunks: Vec<EntityDrawChunk> = Vec::new();
    for (_, _, pending) in scored {
        match pending {
            PendingDraw::Tex {
                cid,
                sx0,
                sy0,
                sx1,
                sy1,
                u0,
                v0,
                u1,
                v1,
            } => {
                let quad = |b: &mut SubBatch| {
                    b.push_quad(sx0, sy0, sx1, sy1, u0, v0, u1, v1, [1.0, 1.0, 1.0, 1.0]);
                };
                match chunks.last_mut() {
                    Some(EntityDrawChunk::Textured {
                        character_id,
                        batch,
                    }) if *character_id == cid => {
                        quad(batch);
                    }
                    _ => {
                        let mut batch = SubBatch::default();
                        quad(&mut batch);
                        chunks.push(EntityDrawChunk::Textured {
                            character_id: cid,
                            batch,
                        });
                    }
                }
            }
            PendingDraw::Solid {
                sx0,
                sy0,
                sx1,
                sy1,
                c,
            } => {
                let quad = |b: &mut SubBatch| {
                    b.push_quad(sx0, sy0, sx1, sy1, 0.0, 0.0, 1.0, 1.0, c);
                };
                match chunks.last_mut() {
                    Some(EntityDrawChunk::Solid { batch }) => {
                        quad(batch);
                    }
                    _ => {
                        let mut batch = SubBatch::default();
                        quad(&mut batch);
                        chunks.push(EntityDrawChunk::Solid { batch });
                    }
                }
            }
        }
    }

    chunks
}

/// Screen-space portrait quad for the dialogue box, if `portrait.png` or a character sheet exists.
pub(crate) fn build_dialogue_portrait_batch(
    r: &mut GpuRenderer,
    asset_store: &mut AssetStore,
    theme: &UiTheme,
    npc_id: &str,
    w: u32,
    h: u32,
    ui_scale: i32,
) -> Option<(String, SubBatch)> {
    let (pl, pt, pr, pb) = layout::dialogue_portrait_rect(w, h, theme, ui_scale);
    let quad = |u0: f32, v0: f32, u1: f32, v1: f32| {
        let mut batch = SubBatch::default();
        let c = [1.0_f32, 1.0, 1.0, 1.0];
        batch.push_quad(
            pl as f32, pt as f32, pr as f32, pb as f32, u0, v0, u1, v1, c,
        );
        batch
    };

    if let Some(sheet) = asset_store.get_dialogue_portrait_sheet(npc_id) {
        let key = dialogue_portrait_asset_key(npc_id);
        r.ensure_character(&key, sheet);
        let tw = sheet.width as f32;
        let th = sheet.height as f32;
        let fw = sheet.frame_width as f32;
        let fh = sheet.frame_height as f32;
        let u1 = fw / tw;
        let v1 = fh / th;
        return Some((key, quad(0.0, 0.0, u1, v1)));
    }

    let sheet = asset_store.get_sheet(npc_id)?;
    r.ensure_character(npc_id, sheet);
    let row = render::facing_sprite_row(Direction::Down).min(sheet.rows.saturating_sub(1));
    let col = 0_u32;
    let src_x = col * sheet.frame_width;
    let src_y = row * sheet.frame_height;
    let tw = sheet.width as f32;
    let th = sheet.height as f32;
    let u0 = src_x as f32 / tw;
    let u1 = (src_x + sheet.frame_width) as f32 / tw;
    let v0 = src_y as f32 / th;
    let v1 = (src_y + sheet.frame_height) as f32 / th;
    Some((npc_id.to_string(), quad(u0, v0, u1, v1)))
}
