//! # BangBang — binary entry point
//!
//! **High-level:** Creates the ECS world, sets up the `App` (winit + wgpu + state), and runs the
//! event loop. On first resume we create the window and [`bangbang::gpu::GpuRenderer`]; on
//! `RedrawRequested` we resize if needed, update state, render with the GPU, present; on
//! `about_to_wait` we request another redraw for continuous animation.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use bangbang::constants::{
    DOOR_TRANSITION_COOLDOWN_SECS, OVERWORLD_TOAST_DURATION_SECS,
};
use bangbang::render;
use bangbang::ui::{self, BackpackPanelLines};
use bangbang::config::GameConfig;
use bangbang::save_game;
use bangbang::{assets, ecs, map, map_loader, render_settings, skills, state};
use glam::Vec2;
use hecs::World;
use state::{AppState, InputState, WorldState};
use winit::application::ApplicationHandler;
use winit::event::{Modifiers, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

/// ECS world, high-level app mode, input, story flags, and per-frame timing.
struct GameState {
    world: World,
    app_state: AppState,
    input: InputState,
    story_state: WorldState,
    modifiers: Modifiers,
    last_frame: Option<Instant>,
    #[cfg(feature = "debug")]
    fps_smoothed: f32,
}

/// Current map id, tile data, doors, and door-transition scratch state.
struct MapContext {
    current_map_id: String,
    doors: Vec<bangbang::config::MapDoor>,
    door_cooldown: f32,
    prev_door_overlap: Option<usize>,
    tilemap: Option<map::Tilemap>,
    tileset: Option<assets::LoadedSheet>,
}

/// Window size, title, and render/UI scale factors from config.
struct RenderConfig {
    render_scale: render::RenderScale,
    ui_scale: render::UiScale,
    font_scale: f32,
    window_width: u32,
    window_height: u32,
    window_title: String,
}

/// Loaded theme, dialogue cache, skills, and GPU asset store.
struct Resources {
    asset_store: assets::AssetStore,
    ui_theme: ui::UiTheme,
    dialogue_cache: bangbang::dialogue::ConversationCache,
    skill_registry: skills::SkillRegistry,
}

/// Ephemeral UI derived state (not persisted across maps).
struct TransientUi {
    backpack_lines: Option<BackpackPanelLines>,
    /// Transient banner in overworld: `(message, seconds remaining)`.
    overworld_toast: Option<(String, f32)>,
}

struct App {
    gpu: Option<bangbang::gpu::GpuRenderer>,
    game: GameState,
    map: MapContext,
    render: RenderConfig,
    resources: Resources,
    ui: TransientUi,
}

impl GameState {
    fn player_position(&self) -> Option<Vec2> {
        self.world
            .query::<(&bangbang::ecs::Player, &bangbang::ecs::Transform)>()
            .iter()
            .next()
            .map(|(_, (_, t))| t.position)
    }

    #[cfg(feature = "debug")]
    fn debug_overlay(&self, tilemap: &map::Tilemap) -> bangbang::gpu::DebugOverlay {
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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.render.window_title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.render.window_width as f64,
                self.render.window_height as f64,
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
                    .game
                    .last_frame
                    .replace(Instant::now())
                    .map(|t| t.elapsed().as_secs_f32())
                    .unwrap_or(1.0 / 60.0);

                self.update(dt);

                #[cfg(feature = "debug")]
                {
                    let inst = 1.0 / dt.max(1e-6);
                    self.game.fps_smoothed = self.game.fps_smoothed * 0.9 + inst * 0.1;
                }

                let dialogue = self
                    .game
                    .app_state
                    .dialogue_display_text(&mut self.resources.dialogue_cache);
                let dialogue_npc_id = match &self.game.app_state {
                    AppState::Dialogue { npc_id, .. } => Some(npc_id.as_str()),
                    _ => None,
                };
                let backpack_open = match self.game.app_state {
                    AppState::Overworld { backpack_open, .. } => backpack_open,
                    _ => false,
                };
                let overworld_toast = match &self.game.app_state {
                    AppState::Overworld { .. } => self
                        .ui
                        .overworld_toast
                        .as_ref()
                        .map(|(s, _)| s.as_str()),
                    _ => None,
                };

                #[cfg(feature = "debug")]
                let debug_overlay = Some(
                    self.game
                        .debug_overlay(self.map.tilemap.as_ref().unwrap()),
                );
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

                gpu.draw_frame(bangbang::gpu::FrameContext {
                    tilemap: self.map.tilemap.as_ref().unwrap(),
                    tileset: self.map.tileset.as_ref(),
                    world: &self.game.world,
                    asset_store: &mut self.resources.asset_store,
                    theme: &self.resources.ui_theme,
                    scales: bangbang::gpu::RenderScales {
                        render: self.render.render_scale,
                        ui: self.render.ui_scale.0,
                        font: self.render.font_scale,
                    },
                    ui: bangbang::gpu::UiFrameState {
                        dialogue_message: dialogue.as_deref(),
                        dialogue_npc_id,
                        overworld_toast,
                        backpack_open,
                        backpack_lines: self.ui.backpack_lines.as_ref(),
                    },
                    debug: debug_overlay,
                })
                .expect("gpu draw");
            }
            WindowEvent::ModifiersChanged(m) => {
                self.game.modifiers = m;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.game
                    .input
                    .apply_key_with_modifiers(&event, &self.game.modifiers);
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
    fn apply_map_transition(&mut self, door: &bangbang::config::MapDoor) {
        let Some(carry) = ecs::take_player_carryover(&mut self.game.world) else {
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
        log::info!(
            "map transition: {} -> {}",
            self.map.current_map_id,
            door.to_map
        );
        map_loader::log_startup_tilemap_diagnostics(&door.to_map, &map_data);
        ecs::despawn_all_entities(&mut self.game.world);
        ecs::setup_world(&mut self.game.world, &map_data, door.spawn, Some(carry));
        self.map.doors = map_data.doors.clone();
        self.map.tilemap = Some(map_data.tilemap);
        self.map.tileset = map_data.tileset;
        self.map.current_map_id = door.to_map.clone();
        self.map.door_cooldown = DOOR_TRANSITION_COOLDOWN_SECS;
        self.map.prev_door_overlap = None;
        self.game.app_state = AppState::Overworld {
            last_near_npc: false,
            backpack_open: false,
        };
    }

    fn apply_load_game(&mut self) -> Result<(), save_game::SaveError> {
        let data = save_game::read_save_file()?;
        let map_data = save_game::restore_world_from_save(
            &data,
            &mut self.game.world,
            &mut self.game.story_state,
        )?;
        self.map.current_map_id = data.map_id;
        self.map.doors = map_data.doors.clone();
        self.map.tilemap = Some(map_data.tilemap);
        self.map.tileset = map_data.tileset;
        self.map.door_cooldown = DOOR_TRANSITION_COOLDOWN_SECS;
        self.map.prev_door_overlap = None;
        self.game.app_state = AppState::Overworld {
            last_near_npc: false,
            backpack_open: true,
        };
        if let Some(p) = skills::player_entity(&self.game.world) {
            if let Ok(mut b) = self.game.world.get::<&mut ecs::Backpack>(p) {
                skills::normalize_equipped_weapon(&mut b, &self.resources.skill_registry);
            }
        }
        Ok(())
    }

    fn update(&mut self, dt: f32) {
        if let AppState::Overworld { .. } = self.game.app_state {
            if let Some((_, t)) = &mut self.ui.overworld_toast {
                *t -= dt;
                if *t <= 0.0 {
                    self.ui.overworld_toast = None;
                }
            }
        }

        let tilemap = self.map.tilemap.as_ref().unwrap();
        self.game.app_state.update(
            &mut self.game.world,
            &mut self.game.input,
            &mut self.game.story_state,
            dt,
            tilemap,
            &self.resources.skill_registry,
            &mut self.resources.dialogue_cache,
        );

        if !matches!(
            self.game.app_state,
            AppState::Overworld {
                backpack_open: true,
                ..
            }
        ) {
            self.game.input.take_save_game_request();
            self.game.input.take_load_game_request();
        }

        if let AppState::Overworld {
            backpack_open: true,
            ..
        } = self.game.app_state
        {
            if self.game.input.take_save_game_request() {
                match save_game::capture_save(
                    &self.game.world,
                    &self.map.current_map_id,
                    &self.game.story_state,
                ) {
                    Ok(data) => match save_game::write_save_file(&data) {
                        Ok(()) => {
                            self.ui.overworld_toast = Some((
                                "Game saved.".to_string(),
                                OVERWORLD_TOAST_DURATION_SECS,
                            ));
                        }
                        Err(e) => {
                            self.ui.overworld_toast = Some((
                                format!("Save failed: {e}"),
                                OVERWORLD_TOAST_DURATION_SECS,
                            ));
                        }
                    },
                    Err(e) => {
                        self.ui.overworld_toast = Some((
                            format!("Save failed: {e}"),
                            OVERWORLD_TOAST_DURATION_SECS,
                        ));
                    }
                }
            }
            if self.game.input.take_load_game_request() {
                match self.apply_load_game() {
                    Ok(()) => {
                        self.ui.overworld_toast = Some((
                            "Game loaded.".to_string(),
                            OVERWORLD_TOAST_DURATION_SECS,
                        ));
                    }
                    Err(e) => {
                        self.ui.overworld_toast = Some((
                            format!("Load failed: {e}"),
                            OVERWORLD_TOAST_DURATION_SECS,
                        ));
                    }
                }
            }
        }

        if let AppState::Overworld {
            backpack_open: false,
            ..
        } = self.game.app_state
        {
            if let Some(pos) = self.game.player_position() {
                match state::map_transition::poll_map_door_transition(
                    &self.map.doors,
                    pos,
                    &mut self.game.input,
                    dt,
                    &mut self.map.door_cooldown,
                    &mut self.map.prev_door_overlap,
                    &self.game.story_state,
                ) {
                    state::map_transition::DoorPollResult::Transition(door) => {
                        self.apply_map_transition(&door);
                    }
                    state::map_transition::DoorPollResult::Blocked { message } => {
                        if !message.is_empty() {
                            self.ui.overworld_toast =
                                Some((message, OVERWORLD_TOAST_DURATION_SECS));
                        }
                    }
                    state::map_transition::DoorPollResult::None => {}
                }
            }
        }

        self.ui.backpack_lines = if let AppState::Overworld {
            backpack_open: true,
            ..
        } = self.game.app_state
        {
            Some(bangbang::ui::backpack_panel_lines(
                &self.game.world,
                &self.resources.skill_registry,
            ))
        } else {
            None
        };
    }
}

fn main() {
    env_logger::init();

    let game_config =
        GameConfig::load().expect("failed to load assets/game.json (see GameConfig::load)");

    let render_settings = render_settings::load().expect("failed to load assets/config.json");

    let map_data = map_loader::load_map(&game_config.start_map).unwrap_or_else(|e| {
        panic!(
            "failed to load start map {:?}: {}",
            game_config.start_map, e
        )
    });

    map_loader::log_startup_tilemap_diagnostics(&game_config.start_map, &map_data);

    let mut world = World::new();
    ecs::setup_world(&mut world, &map_data, map_data.player_start, None);

    let skill_registry = skills::SkillRegistry::load_builtins().expect("load assets/skills/*.json");

    if game_config.seed_demo_backpack {
        skills::seed_demo_backpack(&mut world, &skill_registry).expect("seed backpack");
    }

    let ui_theme = ui::load_theme().expect("failed to load assets/ui/theme.json");

    let mut app = App {
        gpu: None,
        game: GameState {
            world,
            app_state: AppState::default(),
            input: InputState::default(),
            story_state: WorldState::default(),
            modifiers: Modifiers::default(),
            last_frame: None,
            #[cfg(feature = "debug")]
            fps_smoothed: 60.0,
        },
        map: MapContext {
            current_map_id: game_config.start_map.clone(),
            doors: map_data.doors.clone(),
            door_cooldown: 0.0,
            prev_door_overlap: None,
            tilemap: Some(map_data.tilemap),
            tileset: map_data.tileset,
        },
        render: RenderConfig {
            render_scale: render::RenderScale(render_settings.render_scale),
            ui_scale: render::UiScale(render_settings.ui_scale),
            font_scale: render_settings.font_scale,
            window_width: render_settings.window_width,
            window_height: render_settings.window_height,
            window_title: game_config.window_title,
        },
        resources: Resources {
            asset_store: assets::AssetStore::new(),
            ui_theme,
            dialogue_cache: bangbang::dialogue::ConversationCache::new(),
            skill_registry,
        },
        ui: TransientUi {
            backpack_lines: None,
            overworld_toast: None,
        },
    };

    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    let _ = event_loop.run_app(&mut app);
}
