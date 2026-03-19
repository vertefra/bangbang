//! # BangBang — binary entry point
//!
//! **High-level:** Creates the ECS world, sets up the `App` (winit + softbuffer + state), and runs
//! the event loop. The app implements `ApplicationHandler`: on first resume we create the window
//! and softbuffer context; on `RedrawRequested` we update state, draw to the buffer, present; on
//! `about_to_wait` we request another redraw for continuous animation.

use bangbang::{assets, ecs, map, map_loader, software, state, ui};
use hecs::World;
use softbuffer::{Context, Surface};
use state::{AppState, InputState, StoryState};
use std::num::NonZeroU32;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

/// Application state owned by the event loop. **Rust:** We use `Option<T>` for window and context
/// because they don't exist until the first `resumed`; the event loop drives when that happens.
struct App {
    window: Option<Window>,
    softbuffer_context: Option<Context<winit::event_loop::OwnedDisplayHandle>>,
    world: World,
    app_state: AppState,
    input: InputState,
    story_state: StoryState,
    last_frame: Option<Instant>,
    tilemap: Option<map::Tilemap>,
    asset_store: assets::AssetStore,
    ui_theme: ui::UiTheme,
}

/// **Rust: `impl Trait for Type`** = implement the `ApplicationHandler` trait for `App`. The event loop
/// calls these methods; we must provide the required functions. This is like implementing an interface.
impl ApplicationHandler for App {
    /// Called when the app becomes active (e.g. first run). We create the window and softbuffer context once.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("BangBang")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
        let window = event_loop.create_window(attrs).expect("create window");
        let display_handle = event_loop.owned_display_handle();
        let context = Context::new(display_handle).expect("softbuffer context");
        self.window = Some(window);
        self.softbuffer_context = Some(context);
    }

    /// Handle window events: close, resize, redraw, keyboard. **Rust:** `match event { ... }` exhaustively
    /// matches on the enum; `_ => {}` catches the rest. `let Some(ref x) = self.foo else { return };` = early return if None.
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
                let Some(ref context) = self.softbuffer_context else { return };
                let Some(ref window) = self.window else { return };
                let mut surface = Surface::new(context, window).expect("softbuffer surface");
                let size = window.inner_size();
                let (Some(w), Some(h)) = (
                    NonZeroU32::new(size.width),
                    NonZeroU32::new(size.height),
                ) else { return };
                surface.resize(w, h).expect("surface resize");

                // Delta time: replace last_frame with now(), use elapsed of previous or 1/60.
                let dt = self
                    .last_frame
                    .replace(Instant::now())
                    .map(|t| t.elapsed().as_secs_f32())
                    .unwrap_or(1.0 / 60.0);
                let tilemap = self.tilemap.as_ref().unwrap();
                self.app_state.update(&mut self.world, &mut self.input, &mut self.story_state, dt, tilemap);

                let mut buffer = surface.buffer_mut().expect("buffer");
                let width = buffer.width();
                let height = buffer.height();
                let dialogue = self.app_state.dialogue_message();
                software::draw(
                    &mut *buffer,
                    width,
                    height,
                    tilemap,
                    &self.world,
                    dialogue.as_deref(),
                    &mut self.asset_store,
                    &self.ui_theme,
                );
                buffer.present().expect("present");
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.apply_key(&event);
            }
            _ => {}
        }
    }

    /// Before the loop waits for events, request a redraw so we keep animating.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

/// **Rust:** `main` is the program entry. We construct the world, run `setup_world`, build `App`, then
/// `run_app` blocks until the loop exits (e.g. on close). `run_app` takes `&mut app` so it can call
/// the `ApplicationHandler` methods with `&mut self`.
fn main() {
    let map_data = map_loader::load_map("intro");
    let mut world = World::new();
    ecs::setup_world(&mut world, &map_data);

    let mut app = App {
        window: None,
        softbuffer_context: None,
        world,
        app_state: AppState::default(),
        input: InputState::default(),
        story_state: StoryState::default(),
        last_frame: None,
        tilemap: Some(map_data.tilemap),
        asset_store: assets::AssetStore::new(),
        ui_theme: ui::load_theme(),
    };

    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    let _ = event_loop.run_app(&mut app);
}
