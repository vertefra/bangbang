//! GPU rendering via **wgpu** (Vulkan/Metal/D3D12): textured quads for tilemap, sprite sheets, UI, font.

mod frame_context;
mod pass_backpack;
mod pass_common;
mod pass_debug;
mod pass_entities;
mod pass_tilemap;
mod pass_ui;
mod renderer;
mod text_atlas;

pub use frame_context::{DebugOverlay, FrameContext, RenderScales, UiFrameState};
pub use pass_common::GpuVertex;
pub use renderer::GpuRenderer;
