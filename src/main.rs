//! # BangBang — binary entry point
//!
//! **High-level:** Creates the ECS world, sets up the `App` (winit + wgpu + state), and runs the
//! event loop. On first resume we create the window and [`bangbang::gpu::GpuRenderer`]; on
//! `RedrawRequested` we resize if needed, update state, render with the GPU, present; on
//! `about_to_wait` we request another redraw for continuous animation.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use bangbang::constants::DOOR_TRANSITION_COOLDOWN_SECS;
use bangbang::render;
use bangbang::ui::{self, BackpackPanelLines};
use bangbang::{assets, ecs, map, map_loader, render_settings, skills, state};
use glam::Vec2;
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
    current_map_id: String,
    doors: Vec<bangbang::config::MapDoor>,
    door_cooldown: f32,
    prev_door_overlap: Option<usize>,
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
    font_scale: f32,
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

                #[cfg(feature = "debug")]
                {
                    let inst = 1.0 / dt.max(1e-6);
                    self.fps_smoothed = self.fps_smoothed * 0.9 + inst * 0.1;
                }

                let dialogue = self
                    .app_state
                    .dialogue_display_text(&mut self.dialogue_cache);
                let dialogue_npc_id = match &self.app_state {
                    AppState::Dialogue { npc_id, .. } => Some(npc_id.as_str()),
                    _ => None,
                };
                let backpack_open = match self.app_state {
                    AppState::Overworld { backpack_open, .. } => backpack_open,
                    _ => false,
                };

                #[cfg(feature = "debug")]
                let debug_overlay = Some(self.debug_overlay());
                #[cfg(not(feature = "debug"))]
                let debug_overlay: Option<bangbang::gpu::DebugOverlay> = None;

                let Some(ref mut gpu) = self.gpu else { return };

                let size = gpu.window().inner_size();
                let (Some(w), Some(h)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };
                gpu.resize(w.get(), h.get());

                gpu.draw_frame(
                    self.tilemap.as_ref().unwrap(),
                    self.tileset.as_ref(),
                    &self.world,
                    dialogue.as_deref(),
                    dialogue_npc_id,
                    backpack_open,
                    &mut self.asset_store,
                    &self.ui_theme,
                    debug_overlay,
                    self.render_scale,
                    self.ui_scale.0,
                    self.font_scale,
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
    #[cfg(feature = "debug")]
    fn debug_overlay(&self) -> bangbang::gpu::DebugOverlay {
        let tilemap = self.tilemap.as_ref().unwrap();
        let fps = self.fps_smoothed;
        let mut lines = Vec::new();
        let Some(p) = self.player_position() else {
            lines.push("pos: (no player)".to_string());
            return bangbang::gpu::DebugOverlay { fps, lines };
        };
        lines.push(format!("world {:.1},{:.1}", p.x, p.y));
        let (tx, ty) = tilemap.tile_coords_for_world(p);
        let in_map = tx >= 0
            && ty >= 0
            && (tx as u32) < tilemap.width
            && (ty as u32) < tilemap.height;
        if in_map {
            let x = tx as u32;
            let y = ty as u32;
            let id = tilemap.tile_at(x, y).expect("in_map implies valid tile index");
            let blocking = tilemap.is_blocking(x, y);
            match tilemap.tile_palette.tiles.get(&id) {
                Some(e) => {
                    let w = if e.walkable { "yes" } else { "no" };
                    let c = e.color;
                    lines.push(format!("tile {tx},{ty}"));
                    lines.push(format!(
                        "id {id} walkable {w} blocking {} rgb {:.2},{:.2},{:.2}",
                        if blocking { "yes" } else { "no" },
                        c[0],
                        c[1],
                        c[2]
                    ));
                }
                None => {
                    lines.push(format!("tile {tx},{ty}"));
                    lines.push(format!(
                        "id {id} walkable missing palette blocking {}",
                        if blocking { "yes" } else { "no" }
                    ));
                }
            }
        } else {
            lines.push(format!("tile {tx},{ty} OOB"));
            lines.push("blocking yes".to_string());
        }
        bangbang::gpu::DebugOverlay { fps, lines }
    }

    fn player_position(&self) -> Option<Vec2> {
        self.world
            .query::<(&bangbang::ecs::Player, &bangbang::ecs::Transform)>()
            .iter()
            .next()
            .map(|(_, (_, t))| t.position)
    }

    fn apply_map_transition(&mut self, door: &bangbang::config::MapDoor) {
        let Some(carry) = ecs::take_player_carryover(&mut self.world) else {
            log::error!("map transition: no player entity");
            return;
        };
        let map_data = match map_loader::load_map(&door.to_map) {
            Ok(d) => d,
            Err(e) => {
                log::error!("map transition: failed to load {}: {}", door.to_map, e);
                return;
            }
        };
        log::info!("map transition: {} -> {}", self.current_map_id, door.to_map);
        map_loader::log_startup_tilemap_diagnostics(&door.to_map, &map_data);
        ecs::despawn_all_entities(&mut self.world);
        ecs::setup_world(&mut self.world, &map_data, door.spawn, Some(carry));
        self.doors = map_data.doors.clone();
        self.tilemap = Some(map_data.tilemap);
        self.tileset = map_data.tileset;
        self.current_map_id = door.to_map.clone();
        self.door_cooldown = DOOR_TRANSITION_COOLDOWN_SECS;
        self.prev_door_overlap = None;
        self.app_state = AppState::Overworld {
            last_near_npc: false,
            backpack_open: false,
        };
    }

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

        if let AppState::Overworld {
            backpack_open: false,
            ..
        } = self.app_state
        {
            if let Some(pos) = self.player_position() {
                if let Some(door) = state::map_transition::poll_map_door_transition(
                    &self.doors,
                    pos,
                    &mut self.input,
                    dt,
                    &mut self.door_cooldown,
                    &mut self.prev_door_overlap,
                ) {
                    self.apply_map_transition(&door);
                }
            }
        }

        self.backpack_lines = if let AppState::Overworld {
            backpack_open: true,
            ..
        } = self.app_state
        {
            Some(bangbang::ui::backpack_panel_lines(
                &self.world,
                &self.skill_registry,
            ))
        } else {
            None
        };
    }
}

fn main() {
    env_logger::init();

    let render_settings = render_settings::load().expect("failed to load assets/config.json");

    let map_data = map_loader::load_map("mumhome.secondFloor")
        .expect("failed to load mumhome.secondFloor map (assets/maps/mumhome.secondFloor.map/)");

    map_loader::log_startup_tilemap_diagnostics("mumhome.secondFloor", &map_data);

    let mut world = World::new();
    ecs::setup_world(&mut world, &map_data, map_data.player_start, None);

    let skill_registry = skills::SkillRegistry::load_builtins().expect("load assets/skills/*.json");

    skills::seed_demo_backpack(&mut world, &skill_registry).expect("seed backpack");

    let ui_theme = ui::load_theme().expect("failed to load assets/ui/theme.json");

    let mut app = App {
        gpu: None,
        world,
        app_state: AppState::default(),
        input: InputState::default(),
        story_state: WorldState::default(),
        current_map_id: "mumhome.secondFloor".into(),
        doors: map_data.doors.clone(),
        door_cooldown: 0.0,
        prev_door_overlap: None,
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
        font_scale: render_settings.font_scale,
        window_width: render_settings.window_width,
        window_height: render_settings.window_height,
        modifiers: Modifiers::default(),
        backpack_lines: None,
    };

    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    let _ = event_loop.run_app(&mut app);
}
