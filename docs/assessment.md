# BangBang — Architecture Assessment

> Full assessment of pattern correctness, scalability, and extensibility.
> Each issue includes **observations** from the current code and **implementation suggestions** with concrete diffs.

---

## Issue 1 · `gpu/renderer.rs` is a 1,719-line Monolith

### Observations

- [renderer.rs](../src/gpu/renderer.rs) contains **all** GPU rendering: tilemap, entities, UI (HP bar, dialogue, backpack, toast), debug HUD, and the top-level `draw_frame` orchestrator.
- `draw_ui_pass` alone is ~430 lines — it builds quads for the HP bar, dialogue panel, backpack (three sections: usable/weapons/passives), overworld toast, and skill icons.
- `draw_entities_pass` (~200 lines) handles Y-sort depth, sprite sheet animation frames, and solid-color fallbacks.
- Three methods carry `#[allow(clippy::too_many_arguments)]`, suppressing the linter instead of fixing the shape.
- Adding **any** new UI panel (shop, quest log, minimap, cutscene) means editing this single file, creating merge conflicts and cognitive overload.

### Implementation Suggestion

Split into pass modules. Each pass receives a shared `PassContext` with device/queue/atlas references and writes into `SubBatch` outputs that `draw_frame` assembles.

```text
src/gpu/
├── mod.rs              # re-exports GpuRenderer, DebugOverlay
├── renderer.rs         # GpuRenderer struct, draw_frame orchestrator, pipeline setup
│                         ~500 lines: new(), resize(), ensure_tileset/character, upload, draw_frame
├── pass_tilemap.rs     # draw_tilemap_pass (wang autotile, solid color, sheet lookup)
├── pass_entities.rs    # draw_entities_pass (Y-sort, animation, PendingDraw)
├── pass_ui.rs          # draw_ui_pass split: HP bar, dialogue box, toast
├── pass_backpack.rs    # backpack panel (usable/weapon/passive slots, skill icons)
├── pass_debug.rs       # draw_debug_pass (feature-gated)
├── text_atlas.rs       # (unchanged)
├── color.rs            # (unchanged, or merged — see Issue 4)
└── shader.wgsl         # (unchanged)
```

Each pass is a free function or method on a small struct:

```rust
// src/gpu/pass_ui.rs
pub(crate) fn draw_ui(
    ctx: &mut PassContext<'_>,      // device, queue, atlas, sampler, screen dims
    theme: &UiTheme,
    ui: &UiFrameState<'_>,         // dialogue_message, toast, player_hp, etc.
    white_over: &mut SubBatch,
    font: &mut SubBatch,
) { ... }
```

`draw_frame` becomes a thin orchestrator:

```rust
pub fn draw_frame(&mut self, frame: &FrameContext<'_>) -> Result<(), String> {
    let ctx = self.pass_context();
    pass_tilemap::draw(&mut ctx, frame.tilemap, frame.tileset, ...);
    let entity_chunks = pass_entities::draw(&mut ctx, frame.world, ...);
    pass_ui::draw(&mut ctx, frame.theme, &frame.ui, ...);
    pass_debug::draw(&mut ctx, frame.debug, ...);
    // assemble batches, upload, render pass, present
}
```

**Effort:** Medium (mechanical moves, no logic changes).
**Test strategy:** Existing visual output should be pixel-identical — compare screenshots before/after.

---

## Issue 2 · `draw_frame` Takes 14 Positional Arguments

### Observations

Current signature in [renderer.rs:1443–1459](../src/gpu/renderer.rs):

```rust
pub fn draw_frame(
    &mut self,
    tilemap: &Tilemap,
    tileset: Option<&LoadedSheet>,
    world: &World,
    dialogue_message: Option<&str>,
    dialogue_npc_id: Option<&str>,
    overworld_toast: Option<&str>,
    backpack_open: bool,
    asset_store: &mut AssetStore,
    theme: &UiTheme,
    debug_overlay: Option<DebugOverlay>,
    render_scale: RenderScale,
    ui_scale: u32,
    font_scale: f32,
    panel_lines: Option<&crate::ui::BackpackPanelLines>,
) -> Result<(), String>
```

- The call site in `main.rs` (lines 130–146) mirrors this — 14 arguments passed in order. Swapping two accidentally compiles if they share a type (e.g., `render_scale` and `font_scale` are both `f32`-likes).
- Every new `AppState` feature adds arguments here. `Duel` HUD, shop overlay, cutscene letterbox — each one widens both the signature and the call site.

### Implementation Suggestion

Group into a `FrameContext` struct, created in `main.rs` each frame:

```rust
// src/gpu/frame_context.rs  (new file)

/// Per-frame data passed from App::update to GpuRenderer::draw_frame.
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
    pub render: f32,
    pub ui: u32,
    pub font: f32,
}

/// All UI-layer data for one frame.
pub struct UiFrameState<'a> {
    pub dialogue_message: Option<&'a str>,
    pub dialogue_npc_id: Option<&'a str>,
    pub overworld_toast: Option<&'a str>,
    pub backpack_open: bool,
    pub backpack_lines: Option<&'a BackpackPanelLines>,
}
```

`draw_frame` becomes:

```rust
pub fn draw_frame(&mut self, frame: FrameContext<'_>) -> Result<(), String> { ... }
```

Call site in `main.rs`:

```rust
gpu.draw_frame(FrameContext {
    tilemap: self.tilemap.as_ref().unwrap(),
    tileset: self.tileset.as_ref(),
    world: &self.world,
    asset_store: &mut self.asset_store,
    theme: &self.ui_theme,
    scales: RenderScales {
        render: self.render_scale.0,
        ui: self.ui_scale.0,
        font: self.font_scale,
    },
    ui: UiFrameState {
        dialogue_message: dialogue.as_deref(),
        dialogue_npc_id,
        overworld_toast,
        backpack_open,
        backpack_lines: self.backpack_lines.as_ref(),
    },
    debug: debug_overlay,
})?;
```

Named fields eliminate argument-order bugs, and adding new UI state is a field addition, not a signature change.

**Effort:** Small (struct + one call site change).

---

## Issue 3 · The `App` God Struct (25+ Fields)

### Observations

The `App` struct in [main.rs:26–54](../src/main.rs) holds:

| Category | Fields |
|----------|--------|
| GPU | `gpu` |
| ECS | `world` |
| State machines | `app_state`, `input`, `story_state` |
| Map context | `current_map_id`, `doors`, `door_cooldown`, `prev_door_overlap`, `tilemap`, `tileset` |
| Rendering config | `render_scale`, `ui_scale`, `font_scale`, `window_width`, `window_height` |
| Resources | `asset_store`, `ui_theme`, `dialogue_cache`, `skill_registry` |
| Frame timing | `last_frame`, `fps_smoothed` |
| Transient UI | `backpack_lines`, `overworld_toast` |
| Input modifiers | `modifiers` |

- `App::update()` accesses all categories. Testing it requires creating a full `App`.
- Adding a new subsystem (audio, save/load, scene manager) means adding more fields here.
- Two developers working on different systems (e.g., audio + shop) will always conflict on this struct definition.

### Implementation Suggestion

Group into domain sub-structs:

```rust
struct App {
    gpu: Option<GpuRenderer>,
    world: World,
    game: GameState,
    map: MapContext,
    rendering: RenderConfig,
    resources: Resources,
    input: InputState,
    modifiers: Modifiers,
    transient: TransientUi,
    last_frame: Option<Instant>,
    #[cfg(feature = "debug")]
    fps_smoothed: f32,
}

/// Active game state machines.
struct GameState {
    app_state: AppState,
    story_state: WorldState,
    dialogue_cache: dialogue::ConversationCache,
}

/// Everything tied to the current map.
struct MapContext {
    current_map_id: String,
    doors: Vec<MapDoor>,
    door_cooldown: f32,
    prev_door_overlap: Option<usize>,
    tilemap: Option<map::Tilemap>,
    tileset: Option<assets::LoadedSheet>,
}

/// Scale factors and window dimensions.
struct RenderConfig {
    render_scale: render::RenderScale,
    ui_scale: render::UiScale,
    font_scale: f32,
    window_width: u32,
    window_height: u32,
}

/// Shared, long-lived caches and registries.
struct Resources {
    asset_store: assets::AssetStore,
    ui_theme: ui::UiTheme,
    skill_registry: skills::SkillRegistry,
}

/// Ephemeral per-frame UI data.
struct TransientUi {
    backpack_lines: Option<BackpackPanelLines>,
    overworld_toast: Option<(String, f32)>,
}
```

Method signatures become narrower:

```rust
fn apply_map_transition(&mut self, door: &MapDoor) {
    // Only touches self.map, self.world, self.game.app_state
}
```

**Effort:** Medium (struct reorganization, update all field accesses).

---

## Issue 4 · Duplicated `color.rs` Modules

### Observations

Two color utility modules:
- ~~`src/gpu/color.rs`~~ — merged into [src/render/color.rs](../src/render/color.rs) (`packed_rgb_to_linear`, `sprite_color_to_linear`)
- [src/render/color.rs](../src/render/color.rs) — `to_u32` sRGB helper

The renderer calls across the boundary: `crate::render::to_u32(rgb[0], rgb[1], rgb[2])` from inside `gpu/renderer.rs`. The functions serve the same domain (color space conversions for the GPU pipeline).

### Implementation Suggestion

**Option A** — Merge into `src/render/color.rs`, delete `src/gpu/color.rs` (**done**):

```diff
 // src/gpu/mod.rs
-pub mod color;
 pub mod renderer;
 pub mod text_atlas;
```

Move `packed_rgb_to_linear` and `sprite_color_to_linear` into `src/render/color.rs`. Update renderer imports:

```diff
-use super::color::{packed_rgb_to_linear, sprite_color_to_linear};
+use crate::render::color::{packed_rgb_to_linear, sprite_color_to_linear};
```

**Option B** — If `render/` is meant to stay GPU-agnostic, rename `gpu/color.rs` to `gpu/color_convert.rs` with a doc comment explaining why it's separate and add a lint deny for `gpu::color` importing from `render::color`.

Option A is simpler and preferred.

**Effort:** Trivial (move 2 functions, update imports, delete file).

---

## Issue 5 · String-Typed Dialogue Effects and Conditions

### Observations

Effects in [dialogue/mod.rs:77–96](../src/dialogue/mod.rs) and conditions in [dialogue/tree.rs:120–142](../src/dialogue/tree.rs) parse raw strings at runtime:

```rust
fn apply_effect(effect: &str, world_state: &mut WorldState) {
    if let Some(flag) = effect.strip_prefix("set_flag:") { ... }
    if let Some(path) = effect.strip_prefix("set_path:") { ... }
    if let Some(id) = effect.strip_prefix("start_quest:") { ... }
    if let Some(id) = effect.strip_prefix("complete_quest:") { ... }
    log::warn!("dialogue effect has unknown prefix: {:?}", effect);
}
```

- A JSON typo `"set_fleg:met"` silently logs a warning and does nothing — the content creator has no fast feedback.
- The same prefix-parsing pattern is duplicated between effects and conditions.
- Adding a new operation (e.g., `give_item:`, `play_sound:`, `teleport:`) requires editing two functions and remembering to update both.

### Implementation Suggestion

Define typed enums with serde custom deserialization:

```rust
// src/dialogue/effects.rs (new file)

#[derive(Debug, Clone)]
pub enum DialogueEffect {
    SetFlag(String),
    SetPath(String),
    StartQuest(String),
    CompleteQuest(String),
}

impl DialogueEffect {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if let Some(v) = s.strip_prefix("set_flag:") { return Ok(Self::SetFlag(v.trim().into())); }
        if let Some(v) = s.strip_prefix("set_path:") { return Ok(Self::SetPath(v.trim().into())); }
        if let Some(v) = s.strip_prefix("start_quest:") { return Ok(Self::StartQuest(v.trim().into())); }
        if let Some(v) = s.strip_prefix("complete_quest:") { return Ok(Self::CompleteQuest(v.trim().into())); }
        Err(format!("unknown dialogue effect: {:?}", s))
    }

    pub fn apply(&self, world_state: &mut WorldState) {
        match self {
            Self::SetFlag(f) => world_state.set_flag(f),
            Self::SetPath(p) => world_state.choose_path(p),
            Self::StartQuest(id) => world_state.start_quest(id),
            Self::CompleteQuest(id) => world_state.complete_quest(id),
        }
    }
}
```

Parse at load time (in `Conversation::from_json`), not at runtime. Failed parse returns `Result::Err` and halts loading — catching typos immediately.

```rust
// In tree.rs Node deserialization:
pub struct Node {
    // ...
    pub effects: Vec<DialogueEffect>,  // was Vec<String>
}
```

Apply the same pattern for `DialogueCondition`.

**Effort:** Small (new file, change `Vec<String>` to `Vec<DialogueEffect>`, update callers).

---

## Issue 6 · `SkillRegistry` Lacks Iteration / Listing

### Observations

[skills/registry.rs](../src/skills/registry.rs) exposes only `get(&self, id: &str) -> Option<&SkillDef>`. Any system needing to list skills must guess IDs or break encapsulation.

Affected future systems: shop UI, loot tables, "grant all" debug commands, skill-tree menus, save/load (serialize owned skill IDs → validate on load).

### Implementation Suggestion

```rust
// src/skills/registry.rs — additions

impl SkillRegistry {
    // ... existing get() ...

    /// Number of loaded skill definitions.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// True if no skills are loaded.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// Whether a skill with this id exists.
    pub fn contains(&self, id: &str) -> bool {
        self.defs.contains_key(id)
    }

    /// Iterate over all `(id, def)` pairs. Order is not guaranteed.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SkillDef)> {
        self.defs.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// All loaded skill ids. Order is not guaranteed.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.defs.keys().map(String::as_str)
    }
}
```

**Effort:** Trivial (5 method stubs, no behavior changes).

---

## Issue 7 · `AppState::update()` Monolithic Match

### Observations

[state/app.rs:47–195](../src/state/app.rs) — the `update` method is 150 lines with a single `match self` that handles all three states:

- **Overworld** (~80 lines): backpack toggle, movement, NPC proximity, dialogue resolution with 3 fallback branches, cache insertion.
- **Dialogue** (~50 lines): typewriter advance, confirm → advance/close.
- **Duel** (1 line): empty `{}`.

The Overworld arm's NPC→dialogue resolution is deeply nested (6 levels of `if let` / `match`). Adding a 4th state (e.g., `Shop`) means another 50–100 line arm in this already dense function.

### Implementation Suggestion

Delegate to per-state methods and extract the dialogue resolution:

```rust
impl AppState {
    pub fn update(&mut self, /* params */) {
        match self {
            AppState::Overworld { .. } => self.update_overworld(/* params */),
            AppState::Dialogue { .. } => self.update_dialogue(/* params */),
            AppState::Duel => {}
        }
    }

    fn update_overworld(&mut self, /* params */) {
        // backpack toggle + movement logic
        // ...
        if let Some(interaction) = trigger {
            if let Some(new_state) = resolve_npc_dialogue(
                &interaction, dialogue_cache, world_state,
            ) {
                *self = new_state;
            }
        }
    }

    fn update_dialogue(&mut self, /* params */) {
        // typewriter + advance logic
    }
}

/// Resolve which dialogue state to enter for an NPC interaction.
/// Returns None if no line can be shown.
fn resolve_npc_dialogue(
    interaction: &NpcInteraction,
    cache: &mut ConversationCache,
    world_state: &mut WorldState,
) -> Option<AppState> {
    // Consolidates the 40-line fallback chain into a clear function
}
```

This gives each state a focused method and makes the NPC→dialogue resolution independently testable.

**Effort:** Small (extract methods, no logic changes).

---

## Issue 8 · No `Error` Trait Impl for `MapLoadError`

### Observations

[map_loader.rs:154–173](../src/map_loader.rs) defines `MapLoadError` with `Display` but without `std::error::Error`:

```rust
impl std::fmt::Display for MapLoadError { ... }
// Missing: impl std::error::Error for MapLoadError {}
```

This prevents:
- Using `Box<dyn Error>` for generic error handling.
- Interop with `anyhow`, `thiserror`, or other error crates if adopted later.
- Chaining with `?` in contexts that expect `impl Error`.

### Implementation Suggestion

One line:

```rust
impl std::error::Error for MapLoadError {}
```

Or, if the `Io` variant should chain to its source:

```rust
impl std::error::Error for MapLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e, _) => Some(e),
            Self::Json(e, _) => Some(e),
            _ => None,
        }
    }
}
```

**Effort:** Trivial (1–10 lines).

---

## Issue 9 · `SkillRegistry` Silently Accepts Empty Directories

### Observations

[skills/registry.rs:18–43](../src/skills/registry.rs):

```rust
match std::fs::read_dir(&skills_dir) {
    Ok(entries) => { /* load each .json */ }
    Err(e) => {
        log::warn!("failed to read skills directory ...");
        // continues — returns Ok(Self { defs: empty })
    }
}
if defs.is_empty() {
    log::warn!("no skills loaded from {}", ...);
}
Ok(Self { defs })  // always Ok, even with 0 skills
```

This violates antipattern #1 ("Silent Fallback"): a missing or empty `assets/skills/` directory starts the game with zero skills and no crash. The player sees an empty backpack with no indication of a deployment error.

### Implementation Suggestion

```rust
pub fn load_builtins() -> Result<Self, String> {
    let skills_dir = crate::paths::asset_root().join("skills");

    let entries = std::fs::read_dir(&skills_dir)
        .map_err(|e| format!("skills: cannot read {}: {}", skills_dir.display(), e))?;

    let mut defs = HashMap::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                let def = SkillDef::load(id)?;
                defs.insert(id.to_string(), def);
            }
        }
    }

    if defs.is_empty() {
        return Err(format!(
            "skills: no .json files found in {}",
            skills_dir.display()
        ));
    }

    Ok(Self { defs })
}
```

**Effort:** Trivial (change `log::warn` to `return Err`).

---

## Issue 10 · Hardcoded Demo Content in `main()`

### Observations

[main.rs:320–330](../src/main.rs):

```rust
let map_data = map_loader::load_map("mumhome.secondFloor").expect(...);
skills::seed_demo_backpack(&mut world, &skill_registry).expect("seed backpack");
```

- The starting map is a string literal.
- Demo inventory seeding is unconditional.
- No concept of a "game config" or "new game" setup that could be swapped for a title screen, save/load, or testing with a different starting map.

### Implementation Suggestion

Create `assets/game.json`:

```json
{
  "start_map": "mumhome.secondFloor",
  "seed_demo_backpack": true,
  "window_title": "BangBang"
}
```

Load it in `main()`:

```rust
// src/config.rs — addition
#[derive(Debug, Deserialize)]
pub struct GameConfig {
    pub start_map: String,
    #[serde(default)]
    pub seed_demo_backpack: bool,
    #[serde(default = "default_window_title")]
    pub window_title: String,
}

fn default_window_title() -> String { "BangBang".into() }

impl GameConfig {
    pub fn load() -> Result<Self, String> {
        let path = crate::paths::asset_root().join("game.json");
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("game config: {}: {}", path.display(), e))?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("game config: {}: {}", path.display(), e))
    }
}
```

Then in `main()`:

```rust
let game_cfg = config::GameConfig::load().expect("failed to load assets/game.json");
let map_data = map_loader::load_map(&game_cfg.start_map).expect("failed to load start map");
if game_cfg.seed_demo_backpack {
    skills::seed_demo_backpack(&mut world, &skill_registry).expect("seed backpack");
}
```

**Effort:** Small (new struct + JSON file, update `main()`).

---

## Issue 11 · `node_lines()` Allocates a `Vec` Every Call

### Observations

[dialogue/tree.rs:77–89](../src/dialogue/tree.rs):

```rust
pub fn node_lines(&self, node_id: &str) -> Vec<&str> {
    let node = match self.nodes.get(node_id) {
        Some(n) => n,
        None => return vec![],
    };
    if let Some(ref s) = node.line {
        return vec![s.as_str()];
    }
    if !node.lines.is_empty() {
        return node.lines.iter().map(String::as_str).collect();
    }
    vec![]
}
```

Called from:
- `current_display()` — every frame during dialogue (typewriter)
- `line_count()` — called from `advance()` and `entry_point()`
- `advance()` — on confirm press

At 60 FPS, `current_display` → `node_lines` allocates a `Vec<&str>` 60 times/second. For the most common single-line case, that's a heap allocation just to return one element.

### Implementation Suggestion

**Option A** — Normalize at load time. When deserializing `Node`, merge `line` into `lines`:

```rust
impl Node {
    /// Called once after deserialization to normalize storage.
    fn normalize(&mut self) {
        if let Some(line) = self.line.take() {
            if self.lines.is_empty() {
                self.lines.push(line);
            }
        }
    }
}
```

Then `node_lines` returns a slice instead of an owned `Vec`:

```rust
pub fn node_lines(&self, node_id: &str) -> &[String] {
    self.nodes
        .get(node_id)
        .map(|n| n.lines.as_slice())
        .unwrap_or(&[])
}
```

And `current_display` becomes:

```rust
pub fn current_display<'a>(conv: &'a Conversation, node_id: &str, line_index: u32) -> Option<&'a str> {
    conv.node_lines(node_id).get(line_index as usize).map(|s| s.as_str())
}
```

**Option B** — Use `SmallVec<[&str; 2]>` if you want to keep the dual `line`/`lines` JSON representation, avoiding heap allocation for the common 1–2 line case.

Option A is cleaner (normalize once, zero-cost access forever).

**Effort:** Small (change return type, add `normalize()` call in `from_json`).
