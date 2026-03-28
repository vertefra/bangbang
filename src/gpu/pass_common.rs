//! Shared types for render passes: vertices, batching, per-frame camera/world scale.

use crate::render::color::packed_rgb_to_linear;
use crate::render;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Default)]
pub(crate) struct SubBatch {
    pub verts: Vec<GpuVertex>,
    pub indices: Vec<u32>,
}

impl SubBatch {
    pub(crate) fn push_quad(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: [f32; 4],
    ) {
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&[
            GpuVertex {
                position: [x0, y0],
                uv: [u0, v0],
                color,
            },
            GpuVertex {
                position: [x1, y0],
                uv: [u1, v0],
                color,
            },
            GpuVertex {
                position: [x1, y1],
                uv: [u1, v1],
                color,
            },
            GpuVertex {
                position: [x0, y1],
                uv: [u0, v1],
                color,
            },
        ]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Camera and letterbox parameters shared by world-space passes (tilemap, entities).
#[derive(Clone, Copy)]
pub(crate) struct PassFrameParams {
    pub cam_x: f32,
    pub cam_y: f32,
    pub half_w: f32,
    pub half_h: f32,
    pub rs: f32,
}

pub(crate) fn theme_rgb(color: [f32; 3]) -> [f32; 4] {
    packed_rgb_to_linear(render::to_u32(color[0], color[1], color[2]))
}
