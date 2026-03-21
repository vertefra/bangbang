//! # BangBang — binary entry point
//!
//! **High-level:** Creates the ECS world, sets up the `App` (winit + wgpu + state), and runs the
//! event loop. On first resume we create the window and [`bangbang::gpu::GpuRenderer`]; on
//! `RedrawRequested` we resize if needed, update state, render with the GPU, present; on
//! `about_to_wait` we request another redraw for continuous animation.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use bangbang::{assets, ecs, map, map_loader, render_settings, skills, state};
use bangbang::render;
use bangbang::ui::{self, BackpackPanelLines};
use hecs::World;
use state::{AppState, InputState, WorldState};
use winit::application::ApplicationHandler;
use winit::event::{Modifiers, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

struct App {
    gpu: Option<bangbang::gpu::GpuRenderer>,
    world: World,
    app_state: AppState,
    input: InputState,
    story_state: WorldState,
    last_frame: Option<Instant>,
    #[cfg(feature = "debug")]
    fps_smoothed: f32,
    tilemap: Option<map::Tilemap>,
    tileset: Option<assets::LoadedSheet>,
    asset_store: assets::AssetStore,
    ui_theme: ui::UiTheme,
    dialogue_cache: bangbang::dialogue::ConversationCache,
    skill_registry: skills::SkillRegistry,
    render_scale: render::RenderScale,
    ui_scale: render::UiScale,
    window_width: u32,
    window_height: u32,
    modifiers: Modifiers,
    backpack_lines: Option<BackpackPanelLines>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("BangBang")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.window_width as f64,
                self.window_height as f64,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let gpu = bangbang::gpu::GpuRenderer::new(window).expect("wgpu renderer");
        self.gpu = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {}
            WindowEvent::RedrawRequested => {
                let dt = self
                    .last_frame
                    .replace(Instant::now())
                    .map(|t| t.elapsed().as_secs_f32())
                    .unwrap_or(1.0 / 60.0);

                self.update(dt);

                let Some(ref mut gpu) = self.gpu else { return };

                let size = gpu.window().inner_size();
                let (Some(w), Some(h)) = (
                    NonZeroU32::new(size.width),
                    NonZeroU32::new(size.height),
                ) else { return };
                gpu.resize(w.get(), h.get());

                #[cfg(feature = "debug")]
                {
                    let inst = 1.0 / dt.max(1e-6);
                    self.fps_smoothed = self.fps_smoothed * 0.9 + inst * 0.1;
                }

                let dialogue = self.app_state.dialogue_message(&mut self.dialogue_cache);
                let backpack_open = match self.app_state {
                    AppState::Overworld { backpack_open, .. } => backpack_open,
                    _ => false,
                };
                
                #[cfg(feature = "debug")]
                let fps_overlay = Some(self.fps_smoothed);
                #[cfg(not(feature = "debug"))]
                let fps_overlay: Option<f32> = None;


                gpu
                    .draw_frame(
                        self.tilemap.as_ref().unwrap(),
                        self.tileset.as_ref(),
                        &self.world,
                        dialogue.as_deref(),
                        backpack_open,
                        &self.skill_registry,
                        &mut self.asset_store,
                        &self.ui_theme,
                        fps_overlay,
                        self.render_scale,
                        self.ui_scale.0,
                        self.backpack_lines.as_ref(),
                    )
                    .expect("gpu draw");
            }
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.apply_key_with_modifiers(&event, &self.modifiers);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref gpu) = self.gpu {
            gpu.window().request_redraw();
        }
    }
}

impl App {
    fn update(&mut self, dt: f32) {
        let tilemap = self.tilemap.as_ref().unwrap();
        self.app_state.update(
            &mut self.world,
            &mut self.input,
            &mut self.story_state,
            dt,
            tilemap,
            &self.skill_registry,
            &mut self.dialogue_cache,
        );
        
        self.backpack_lines = if let AppState::Overworld { backpack_open: true, .. } = self.app_state {
            Some(bangbang::ui::backpack_panel_lines(&self.world, &self.skill_registry))
        } else {
            None
        };
    }
}

fn main() {
    env_logger::init();
    
    let render_settings = render_settings::load()
        .expect("failed to load assets/config.json");
    
    let map_data = map_loader::load_map("intro")
        .expect("failed to load intro map (assets/maps/intro.map/)");
        
    map_loader::log_startup_tilemap_diagnostics(&map_data);
    
    let mut world = World::new();
    ecs::setup_world(&mut world, &map_data);
    
    let skill_registry = skills::SkillRegistry::load_builtins()
        .expect("load assets/skills/*.json");
        
    skills::seed_demo_backpack(&mut world, &skill_registry)
        .expect("seed backpack");

    let ui_theme = ui::load_theme()
        .expect("failed to load assets/ui/theme.json");

    let mut app = App {
        gpu: None,
        world,
        app_state: AppState::default(),
        input: InputState::default(),
        story_state: WorldState::default(),
        last_frame: None,
        #[cfg(feature = "debug")]
        fps_smoothed: 60.0,
        tilemap: Some(map_data.tilemap),
        tileset: map_data.tileset,
        asset_store: assets::AssetStore::new(),
        ui_theme,
        dialogue_cache: bangbang::dialogue::ConversationCache::new(),
        skill_registry,
        render_scale: render::RenderScale(render_settings.render_scale),
        ui_scale: render::UiScale(render_settings.ui_scale),
        window_width: render_settings.window_width,
        window_height: render_settings.window_height,
        modifiers: Modifiers::default(),
        backpack_lines: None,
    };

    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    let _ = event_loop.run_app(&mut app);
}
