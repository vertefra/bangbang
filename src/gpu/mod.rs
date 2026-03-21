//! GPU rendering via **wgpu** (Vulkan/Metal/D3D12): textured quads for tilemap, sprite sheets, UI, font.

mod color;
mod renderer;

pub use renderer::{GpuRenderer, GpuVertex};
