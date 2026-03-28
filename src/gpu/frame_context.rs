//! Per-frame inputs for [`crate::gpu::GpuRenderer::draw_frame`].

use crate::assets::{AssetStore, LoadedSheet};
use crate::map::Tilemap;
use crate::render::RenderScale;
use crate::ui::{BackpackPanelLines, UiTheme};
use hecs::World;

/// Developer HUD when built with `--features debug`: smoothed FPS plus lines built in `main` (world position, tile grid, palette properties).
#[derive(Clone, Debug)]
pub struct DebugOverlay {
    pub fps: f32,
    pub lines: Vec<String>,
}

/// Per-frame data passed from App to [`crate::gpu::GpuRenderer::draw_frame`].
pub struct FrameContext<'a> {
    pub tilemap: &'a Tilemap,
    pub tileset: Option<&'a LoadedSheet>,
    pub world: &'a World,
    pub asset_store: &'a mut AssetStore,
    pub theme: &'a UiTheme,
    pub scales: RenderScales,
    pub ui: UiFrameState<'a>,
    pub debug: Option<DebugOverlay>,
}

pub struct RenderScales {
    pub render: RenderScale,
    pub ui: u32,
    pub font: f32,
}

/// UI-layer data for one frame.
pub struct UiFrameState<'a> {
    pub dialogue_message: Option<&'a str>,
    pub dialogue_npc_id: Option<&'a str>,
    pub overworld_toast: Option<&'a str>,
    pub backpack_open: bool,
    pub backpack_lines: Option<&'a BackpackPanelLines>,
}
