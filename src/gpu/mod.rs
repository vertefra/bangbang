//! GPU rendering via **wgpu** (Vulkan/Metal/D3D12): textured quads for tilemap, sprite sheets, UI, font.

mod color;
mod renderer;
mod text_atlas;

pub use renderer::{DebugOverlay, GpuRenderer, GpuVertex};
