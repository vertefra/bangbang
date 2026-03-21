# Proposed Changes

> **UPDATE (Refactor Complete):** All `PC-*` proposals in this document have been successfully **IMPLEMENTED** in the engine. This document remains for historical context.

> Machine-first, unambiguous language. Each proposal references the finding ID from `current-state.md`.

---

## PC-01 · Cache Loaded Conversations (fixes CS-01)

**What:** Add a `HashMap<String, Conversation>` cache to `App`. `dialogue::load_or_fallback` is called once per conversation entry; subsequent reads use the cache.

**Where to change:**
- `dialogue/loader.rs` — no change. Keep `load`/`load_or_fallback` as pure I/O functions.
- `state/app.rs` — store `conversation: Option<Conversation>` inside `AppState::Dialogue`. Populate on transition to `Dialogue`. Use stored reference in `update()` and `dialogue_message()`. Remove per-frame `load_or_fallback` calls.

**Expected result:** Zero disk I/O during dialogue after initial load. Current display and advance operate on the in-memory `Conversation`.

---

## PC-02 · Replace Silent Fallbacks With Explicit Errors (fixes CS-02)

**What:** Change all data-loading functions to return `Result<T, LoadError>`. Propagate errors to `main()`. On error: log structured context (file path, parse error, field name) and either panic at startup (for critical data like the intro map) or show an in-game error screen.

**Where to change:**
- `map_loader::load_map` → return `Result<MapData, MapLoadError>`.
- `dialogue::load` → already returns `Option`. OK.
- `dialogue::load_or_fallback` → keep for intentional fallbacks; add `eprintln!` with map/npc context so missing files are visible.
- `render_settings::load` → return `Result<RenderSettings, ConfigError>`. Panic in `main` if critical config is broken.
- `ui/theme.rs::load_theme` → return `Result`. Fallback to default is acceptable (theme is non-critical), but always log at `WARN`.
- `map_loader::load_npcs_from_map` → return `Result`. If any NPC file fails, include the id in the error so the content author knows which file is broken. Remove `default_npcs()` hard fallback.

**Mitigation for development:** Add a `--strict` CLI flag or a Cargo feature. When enabled: any fallback triggers a panic with the error context. When disabled: log and continue. Default to strict in debug builds.

---

## PC-03 · Auto-Discover Skills From Directory (fixes CS-03)

**What:** Replace `load_builtins()` hardcoded list with filesystem scan of `assets/skills/*.json`. Load all valid files; report errors for invalid ones.

**Where to change:**
- `skills/registry.rs` — replace `let ids = ["sidearm", "beer"]` with `std::fs::read_dir(skills_dir)` → filter `.json` → `SkillDef::load(stem)` for each.

**Expected result:** Adding a new skill = adding a JSON file. No code change required. Matches the documented "data-driven" promise.

---

## PC-04 · Model `backpack_open` as AppState Sub-State (fixes CS-04)

**What:** Extend `AppState::Overworld` to carry sub-mode state:
```
Overworld {
    last_near_npc: bool,
    sub_mode: OverworldSubMode, // Normal, BackpackOpen, PauseMenu, Shop, ...
}
```

**Where to change:**
- `state/app.rs` — add `OverworldSubMode` enum. Move backpack toggle into `AppState::update`.
- `main.rs` — remove `backpack_open: bool` from `App`. Remove backpack toggle/hotkey logic from `RedrawRequested`; delegate to `AppState::update`.
- `gpu/renderer.rs` — `draw_frame` receives `AppState` (or a derived `DrawMode` enum) instead of a separate `backpack_open: bool`.

**Expected result:** `AppState` fully describes the game mode. Adding a new Overworld sub-mode = adding a variant to `OverworldSubMode` + handling it in `update` and `draw_frame`.

---

## PC-05 · Separate Update Tick From Render Tick (fixes CS-05)

**What:** Extract game logic from `RedrawRequested` into a dedicated `App::tick(dt)` method. `RedrawRequested` calls `tick(dt)` then `draw()`.

**Where to change:**
- `main.rs` — create `App::tick(&mut self, dt: f32)` containing: `AppState::update`, backpack logic, input consumption, fps smoothing. `RedrawRequested` handler calls `self.tick(dt)` then `self.draw()`.

**Future benefit:** Enables fixed-timestep update (accumulate dt, tick in fixed increments), headless testing (call `tick` without a window), and replay (feed recorded input into `tick`).

---

## PC-06 · Break Up `software.rs` (fixes CS-06)

**What:** Distribute helpers to their logical modules:

| Current location | Move to | Rationale |
|---|---|---|
| `wang_wall_sheet_index`, `wang_corner_*`, `tilemap_is_binary_collision_only`, `WANG16_SHEET_LUT` | New `map/autotile.rs` or `map/wang.rs` | Tilemap rendering concern |
| `to_u32`, `fill_rect`, `draw_text`, `build_font_atlas_rgba`, `FONT_ATLAS_*` | `ui/primitives.rs` or `ui/draw.rs` | UI drawing primitives |
| `facing_sprite_row` | `ecs/components.rs` as method on `Direction` | Animation/ECS concern |
| `BackpackPanelLines`, `backpack_panel_lines` | `ui/backpack.rs` or `skills/backpack_view.rs` | UI data preparation |

**Where to change:** `software.rs` becomes empty and is removed from `lib.rs`. All importers update to the new module paths.

---

## PC-07 · Decouple Backpack Display From ECS Query (fixes CS-07)

**What:** `backpack_panel_lines` should not query ECS directly. Instead, `App::tick` prepares a `BackpackPanelLines` struct (or a more general `UiState` struct) from the ECS world during the update phase. The renderer receives this pre-computed data.

**Where to change:**
- `main.rs` (or `App::tick`) — after `AppState::update`, compute `BackpackPanelLines` if backpack is open, store as `Option<BackpackPanelLines>` on `App`.
- `gpu/renderer.rs` — `draw_frame` receives `Option<&BackpackPanelLines>` instead of `&World` + `&SkillRegistry` for backpack.

**Expected result:** Renderer has no dependency on `hecs::World` or `SkillRegistry`. Data flows uni-directionally: ECS → update → display data → renderer.

---

## PC-08 · Decompose `draw_frame` (fixes CS-08)

**What:** Split `draw_frame` into composable draw passes:
1. `draw_tilemap(&mut self, tilemap, tileset, render_scale)` — tile quads
2. `draw_entities(&mut self, world, asset_store, render_scale)` — entity quads
3. `draw_ui(&mut self, ui_state, theme, ui_scale)` — dialogue, backpack, HUD
4. `draw_debug(&mut self, fps_overlay, ui_scale)` — FPS overlay

`draw_frame` calls these in order, collects sub-batches, and submits one render pass.

**Where to change:**
- `gpu/renderer.rs` — extract helper methods. Each method operates on sub-batches. `draw_frame` orchestrates the pass.

**Expected result:** Adding a new draw pass (e.g., duel screen, minimap, particle effects) = adding a new method / calling it from `draw_frame`. Each pass is testable and reviewable in isolation.

---

## PC-09 · Add `to_u32` Helper for `[f32; 3]` Arrays (fixes CS-09)

**What:** Add `fn color_to_u32(c: [f32; 3]) -> u32` that calls `to_u32(c[0], c[1], c[2])`. Replace all manual unpacking in `ui/mod.rs` and `ui/backpack.rs`.

**Where to change:**
- `ui/mod.rs` or wherever `to_u32` lives after PC-06.
- All `draw_*` functions in `ui/`.

**Expected result:** 10+ `to_u32(c[0], c[1], c[2])` calls collapse to `color_to_u32(c)`. Theme color changes require no call-site edits.

---

## PC-10 · Deprecate Legacy NPC Config Format (fixes CS-10)

**What:** Add an `eprintln!` warning when `{id}.npc.json` (legacy) is used instead of `{id}.npc/config.json`. In strict mode (per PC-02), make it an error.

**Where to change:**
- `map_loader.rs:174-181` — after successful legacy load, emit: `eprintln!("DEPRECATION: {id}.npc.json used; migrate to {id}.npc/config.json")`.

**Expected result:** Content authors are alerted to migrate. No behavior change in non-strict mode.

---

## PC-11 · Centralize Asset Root (fixes CS-11)

**What:** Create a single `fn asset_root() -> PathBuf` in a shared location (e.g., `config.rs` or a new `paths.rs`). All `assets_dir()` functions call this one function.

**Where to change:**
- New file `src/paths.rs` (or add to `config.rs`): `pub fn asset_root() -> PathBuf`.
- `map_loader.rs`, `render_settings.rs`, `dialogue/loader.rs`, `assets.rs`, `skills/defs.rs` — replace local `assets_dir()` with `crate::paths::asset_root()`.

**Future benefit:** When distribution support is added, only `asset_root()` needs to change (e.g., resolve relative to executable instead of `CARGO_MANIFEST_DIR`).

---

## PC-12 · Remove `dialogue_line` From `Npc` Component (fixes CS-12, CS-13)

**What:** Remove `dialogue_line: String` from `Npc` component and `fallback_line: String` from `AppState::Dialogue`. Replace with: if conversation file is missing, log an error (WARN level) and display a standard "..." or empty-string dialogue, or use a default conversation JSON.

**Where to change:**
- `ecs/components.rs` — remove `dialogue_line` from `Npc`.
- `config.rs` — remove `dialogue_line` from `NpcConfig` and `CharacterNpcConfig`.
- `state/app.rs` — remove `fallback_line` from `AppState::Dialogue`. `dialogue_message()` calls `dialogue::load(conversation_id)` (cached per PC-01); if `None`, return `Some("...")` or `None`.
- `state/overworld.rs` — return `(npc_id, conversation_id)` instead of `(npc_id, conversation_id, fallback_line)`.

**Expected result:** ECS components hold only mutable runtime state. Static content is in data files only.

---

## PC-13 · Add a Log Crate (fixes CS-14)

**What:** Add the `log` crate + a simple backend (e.g., `env_logger` or `simplelog`). Replace all `eprintln!` with `log::warn!`, `log::error!`, `log::info!`, `log::debug!`.

**Where to change:**
- `Cargo.toml` — add `log` and `env_logger`.
- `main.rs` — initialize logger at startup.
- All `eprintln!` call sites — replace with `log::*` macro.

**Expected result:** Log levels filter noise. Developers see all messages; release builds suppress debug. Structured output enables log parsing.

---

## PC-14 · Replace Untyped Tuples With Named Structs (fixes CS-15)

**What:** Define:
```rust
pub struct NpcInteraction {
    pub npc_id: String,
    pub conversation_id: String,
}
```
Return `(Option<NpcInteraction>, bool)` from `overworld::update`. Or further: return a dedicated `OverworldResult` struct.

**Where to change:**
- `state/overworld.rs` — define `NpcInteraction`, return it.
- `state/app.rs` — destructure `NpcInteraction` fields.

---

## PC-15 · Unify CPU and GPU UI Paths or Remove CPU Path (fixes CS-16)

**What:** Two options:

### Option A — Remove CPU UI Path (Recommended)
Delete `ui/panel.rs`, `ui/label.rs`, `ui/bar.rs`, and the CPU `draw_dialogue` / `draw_backpack` / `draw_overworld_hud` / `draw_duel` entry points. Keep `ui/layout.rs` and `ui/theme.rs` (used by GPU). The GPU renderer is the only renderer.

### Option B — Shared UI Abstraction
Create a `UiDrawBackend` trait with `draw_rect(rect, color)`, `draw_text(pos, text, color, scale)`. Implement for CPU buffer and GPU quad batch. UI module calls through the trait. Both paths stay in sync.

**Recommendation:** Option A. The CPU renderer is dead code. Removing it eliminates divergence risk and reduces maintenance surface.

**Where to change (Option A):**
- Delete `src/ui/panel.rs`, `src/ui/label.rs`, `src/ui/bar.rs`.
- `src/ui/mod.rs` — remove `draw_dialogue`, `draw_backpack`, `draw_overworld_hud`, `draw_duel` functions. Keep `pub mod layout`, `pub use theme::*`.
- Update any remaining imports.

---

## PC-16 · Fix Theme Zero-Value Ambiguity (fixes CS-17)

**What:** Use `Option<[f32; 3]>` for color fields in `DialogueThemeFile` / `BackpackThemeFile`. `None` = "use Rust default"; `Some([0,0,0])` = "author chose black".

**Where to change:**
- `ui/theme.rs` — change color fields in file structs to `Option<[f32; 3]>`. In `From` / `merge_backpack`, use `f.field.unwrap_or(default_value)`.

---

## PC-17 · Fix `load_theme` Path Inconsistency (fixes CS-18)

**What:** Use `crate::paths::asset_root().join("ui/theme.json")` (after PC-11) instead of hardcoded `"assets/ui/theme.json"`.

**Where to change:** `ui/theme.rs:177`.

---

## PC-18 · Remove Dead Layout Functions (fixes CS-19)

**What:** Delete `backpack_permanent_title_y` and `backpack_permanent_slot_y` from `ui/layout.rs`. If they are needed later, re-derive from the living functions.

**Where to change:** `ui/layout.rs:108-136`.

---

## PC-19 · Unify Interaction Distance Constant (fixes CS-20)

**What:** Define one constant in a shared location:
```rust
// src/config.rs or src/constants.rs
pub const NPC_INTERACT_RANGE: f32 = 23.0;
```
`state/overworld.rs` and `skills/apply.rs` import it.

**Where to change:**
- New `src/constants.rs` or shared location.
- `state/overworld.rs:14` — replace `INTERACT_DISTANCE`.
- `skills/apply.rs:10` — replace `OPPONENT_RANGE`.

---

## Priority Order

Changes ordered by impact on data-driven scalability and maintenance:

| Priority | ID | Effort | Impact |
|---|---|---|---|
| 1 | PC-01 | Low | Eliminates per-frame disk I/O |
| 2 | PC-03 | Low | Unlocks true data-driven skills |
| 3 | PC-11 | Low | Single asset root for all modules |
| 4 | PC-05 | Medium | Clean update/render separation |
| 5 | PC-04 | Medium | AppState fully describes game mode |
| 6 | PC-06 | Medium | Proper module boundaries |
| 7 | PC-08 | Medium | Manageable GPU renderer |
| 8 | PC-02 | Medium | Explicit error handling |
| 9 | PC-07 | Low | Decouple render from ECS |
| 10 | PC-15 | Low | Remove dead CPU UI code |
| 11 | PC-14 | Low | Type safety for overworld results |
| 12 | PC-12 | Low | Clean ECS components |
| 13 | PC-13 | Low | Proper logging |
| 14 | PC-09 | Trivial | Reduce boilerplate |
| 15 | PC-19 | Trivial | Deduplicate constant |
| 16 | PC-10 | Trivial | Deprecation warning |
| 17 | PC-16 | Low | Theme correctness |
| 18 | PC-17 | Trivial | Path consistency |
| 19 | PC-18 | Trivial | Remove dead code |
