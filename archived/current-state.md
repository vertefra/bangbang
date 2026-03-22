# Historical Audit Archive

> This file is a frozen snapshot of an older audit. The `CS-*` sections below are **not** the current bug list, and many items have since been resolved or refactored. The stale body is preserved intentionally as archival context.

> Treat code locations and behavior descriptions here as historical context only. Use `docs/architecture.md`, `docs/game.md`, `docs/maps.md`, and `docs/ui.md` for the current system description.

---

## CS-01 · Per-Frame Disk I/O for Dialogue

**Location:** `state/app.rs:99`, `state/app.rs:74`, `dialogue/loader.rs:17`

**What happens:** `dialogue::load_or_fallback()` calls `std::fs::read_to_string` + `serde_json::from_str` every time it is invoked. It is called in two hot paths:
1. `AppState::dialogue_message()` — called every frame during `RedrawRequested` to get the display string.
2. `AppState::update()` in the `Dialogue` branch — called every frame, re-parses the conversation on each `confirm_pressed`.

**Why it is a problem:** Disk I/O per frame is orders of magnitude slower than memory access. It also masks data errors — if the file is missing, it silently falls back to `Conversation::one_line()` every frame, and no one notices the file is broken.

**Architectural deviation:** `architecture.md` §Dialogue says "Load from assets/dialogue/{id}.json" — implies load-once. The code loads-every-use.

---

## CS-02 · Silent Fallback Chains Mask Data Errors

**Location:** `map_loader.rs:99-124`, `map_loader.rs:130-136`, `map_loader.rs:166-170`, `dialogue/loader.rs:17-18`, `render_settings.rs:72-96`, `ui/theme.rs:175-194`, `assets.rs:163-167`

**What happens:** Every data-loading function follows the pattern: try read → try parse → on any error → return hardcoded default. No error is propagated to the caller. The only signal is `eprintln!` in some (not all) branches.

**Specific instances:**
| Location | What is silenced |
|---|---|
| `load_map` | Missing `map.json`, bad JSON, wrong tile dimensions, missing palette — all collapse to `fallback_tilemap()` |
| `load_npcs_from_map` | If all NPC config files fail, returns `default_npcs()` (hardcoded "mom" at [100,100]) |
| `load_or_fallback` | Missing dialogue file → one-line conversation from `dialogue_line` field |
| `render_settings::load` | Missing or invalid `config.json` → hardcoded 800×600, scale 2 |
| `load_theme` | Missing or invalid `theme.json` → hardcoded defaults |
| `load_sheet_from_dir` | Missing `sheet.json` → hardcoded (4, 2) grid |

**Why it is a problem:** A content author adds a new map, misspells a filename or uses a wrong field name. The game launches with fallback data. There is no visible error in the game window. The content author concludes "the engine is broken" or ships wrong content without knowing. As content volume grows, this pattern becomes undebuggable.

**AGENTS.md violation:** Rule 3: "explicit_fail > silent/hidden".

---

## CS-03 · Hardcoded Skill Registry List

**Location:** `skills/registry.rs:14-15`

**What happens:** `SkillRegistry::load_builtins()` contains a hardcoded array `["sidearm", "beer"]`. Every new skill requires a code change in this array.

**Architectural deviation:** `architecture.md` and `game.md` state skills are "data-driven" and "new skills can be added via config without code changes." The registry contradicts this.

---

## CS-04 · `backpack_open` State Lives Outside `AppState`

**Location:** `main.rs:33` (`backpack_open: bool` on `App`), `main.rs:83-103`

**What happens:** `AppState` enum is `Overworld | Dialogue | Duel`. Backpack-open is a sub-mode of Overworld but lives as a separate `bool` on the `App` struct. The backpack toggle logic, normalize, hotkeys, and cycle are all in the `RedrawRequested` handler in `main.rs:83-103`.

**Why it is a problem:**
1. `AppState` does not fully describe the game mode. Code must check both `AppState` and `backpack_open` to know what the player sees.
2. The backpack bool is never reset when transitioning from Overworld to Dialogue or Duel. If `backpack_open == true` and the player triggers dialogue, the backpack state leaks (currently not drawn but state is stale).
3. Adding more Overworld sub-modes (shop, pause menu, map, inventory) following this pattern creates a combinatorial explosion of bool flags on `App`.

**Architectural deviation:** `architecture.md` §State: "All modes go through AppState; new modes = new enum variants."

---

## CS-05 · Game Logic in the Redraw Handler

**Location:** `main.rs:67-131` (inside `WindowEvent::RedrawRequested`)

**What happens:** `RedrawRequested` does: compute dt → `AppState::update` → backpack toggle → normalize → hotkeys → weapon cycle → (with `debug` feature) FPS smooth + `DebugOverlay` strings → dialogue message → then `gpu` resize → `gpu.draw_frame`. Game logic (state update, backpack toggle, skill hotkeys) is interleaved with rendering in the same event handler.

**Why it is a problem:**
1. No separation between update tick and render tick. If the game ever needs fixed-timestep physics, replay, or headless testing, the update logic is locked to the frame rate and to the window event loop.
2. Adding more game systems (duel logic, AI, animation, particles) in this handler will make it grow unboundedly.
3. The `about_to_wait` handler requests a redraw every frame, so the update rate is tied to the display refresh rate.

---

## CS-06 · `software.rs` is a Mixed-Concern Utility Bag

**Location:** `software.rs` (317 lines)

**Contents mixed in one file:**
1. Wang autotile logic (tilemap rendering concern)
2. `to_u32` / `fill_rect` / `draw_text` (CPU pixel drawing primitives)
3. `gpu/text_atlas.rs` — `fontdue` raster cache into a dynamic UI atlas (`Rgba8Unorm`, linear sampling)
4. `facing_sprite_row` (ECS/animation concern)
5. `BackpackPanelLines` struct + `backpack_panel_lines` function (UI data preparation concern)
6. `tilemap_is_binary_collision_only` (map rendering concern)

**Why it is a problem:** `software.rs` is imported by `gpu/`, `ui/`, `state/`, and `main.rs`. Every new feature that needs any utility adds to this file. No clear module boundary. The name "software" is misleading — it was originally a CPU renderer but is now a dumping ground for shared helpers.

---

## CS-07 · `backpack_panel_lines` Queries ECS from a Rendering Helper

**Location:** `software.rs:257-316`

**What happens:** `backpack_panel_lines(world, registry)` performs an ECS query (`query::<(&Player, &Backpack)>`) and clones the `Backpack` component to build display strings. This function is called from `gpu/renderer.rs` inside `draw_frame`.

**Why it is a problem:**
1. Rendering code reaches into the ECS world to extract game state. This couples the render layer to ECS internals.
2. The `Backpack` is cloned every frame to produce display strings. The strings are formatted, allocated, and discarded every frame.
3. When the backpack grows (more slots, more skills), this per-frame allocation grows proportionally.

---

## CS-08 · `draw_frame` is a 500-line Monolith

**Location:** `gpu/renderer.rs:505-1006`

**What happens:** `GpuRenderer::draw_frame` is a single function (~500 lines) that handles: tilemap rendering (solid fill path, tileset path, wang autotile path), entity rendering (color rect path, sprite sheet path), UI overlay (dialogue box, backpack panel), optional debug HUD (`DebugOverlay` / `draw_debug_pass`), and the full wgpu render pass submission.

**Why it is a problem:** Adding new visual features (duel screen, HUD, map transitions, particle effects, NPC portraits, minimap) each add 50-200 lines to this function. The function will become unmaintainable.

---

## CS-09 · Duplicated Color Conversion Boilerplate

**Location:** `ui/mod.rs:28-42`, `ui/backpack.rs:25-69`

**What happens:** Every `draw_*` function manually unpacks `[f32; 3]` theme color arrays into `to_u32(c[0], c[1], c[2])` calls. The `draw_backpack` function has 10 successive `to_u32` calls converting theme colors.

**Why it is a problem:** Adding or changing a theme color requires finding every `to_u32` call site. The pattern invites copy-paste errors (wrong index). The conversion could be a method on `UiTheme` or a utility function that takes `[f32; 3]`.

---

## CS-10 · Dual NPC Config Path Without Migration

**Location:** `map_loader.rs:174-181`

**What happens:** `load_character_npc` tries `{id}.npc/config.json` first, then falls back to `{id}.npc.json` (legacy). Both formats are supported indefinitely.

**Why it is a problem:** Content authored in the legacy format works silently. New features that require the folder format (e.g., sprite sheets in `{id}.npc/sheet.png`) will not work with legacy JSON files, but there is no warning. As NPC count grows, some NPCs use one format, some use the other; no tooling flags which ones to migrate.

---

## CS-11 · `CARGO_MANIFEST_DIR` for Asset Paths

**Location:** `map_loader.rs:75`, `render_settings.rs:7`, `dialogue/loader.rs:6`, `assets.rs:8`, `skills/defs.rs:52`

**What happens:** Five different modules each have their own `assets_dir()` function using `env!("CARGO_MANIFEST_DIR")`. This macro embeds a compile-time absolute path.

**Why it is a problem:**
1. The binary only works when run from the machine it was compiled on with the same directory structure.
2. Five copies of the same path logic = five places to update if the asset root changes.
3. Distribution (packaging, deployment) requires all assets at the compile-time path.

---

## CS-12 · `Npc` Component Stores Fallback Dialogue String

**Location:** `ecs/components.rs:71-75`, `ecs/world.rs:37-39`

**What happens:** The `Npc` component stores `dialogue_line: String` — a fallback text displayed when no conversation file exists. This string is set from `CharacterNpcConfig.dialogue_line` during `setup_world` and carried on the entity forever.

**Why it is a problem:**
1. ECS components should hold mutable runtime state, not static fallback content. The dialogue string never changes at runtime.
2. If the NPC's conversation file is later created, the fallback string is still on the entity, wasting memory.
3. The fallback mechanism encourages content authors to rely on `dialogue_line` instead of creating proper conversation JSON files.

---

## CS-13 · `AppState::Dialogue` Stores `fallback_line: String`

**Location:** `state/app.rs:21`

**What happens:** `AppState::Dialogue` contains `fallback_line: String`, cloned from the NPC component on dialogue entry. This string is carried in the state enum variant and passed to `dialogue::load_or_fallback` on every frame and every confirm.

**Why it is a problem:** Same as CS-12 — static content data in runtime state. Combined with CS-01, the fallback_line is used to reconstruct a `Conversation::one_line()` on every single frame during dialogue if the file is missing.

---

## CS-14 · No Centralized Error/Logging System

**Location:** Throughout the codebase

**What happens:** Errors are reported via `eprintln!` with ad-hoc format strings. There is no log crate, no log level filtering, no structured output.

**Why it is a problem:** In a released game, `eprintln!` output disappears unless the player runs from a terminal. There is no way to distinguish content warnings (missing NPC file) from code errors (entity has no Health). As the game grows, the stderr stream becomes noisy and unsearchable.

---

## CS-15 · Overworld Returns Untyped Tuple

**Location:** `state/overworld.rs:40`

**What happens:** `overworld::update` returns `(Option<(String, String, String)>, bool)`. The three strings are `(npc_id, conversation_id, fallback_line)` — no named struct, no type alias.

**Why it is a problem:** Callers must remember the positional meaning of each `String`. Swapping `npc_id` and `conversation_id` compiles silently. Adding a fourth field (e.g., `faction`) changes the tuple shape throughout the call chain.

---

## CS-16 · UI Module Partially Bypassed by GPU Renderer

**Location:** `gpu/renderer.rs:505-1006`, `ui/mod.rs`, `ui/backpack.rs`

**What happens:** The `ui` module has `draw_dialogue`, `draw_backpack`, etc., that operate on CPU pixel buffers (`&mut [u32]`). The GPU renderer (`gpu/renderer.rs`) does NOT call these functions. Instead, `draw_frame` reimplements dialogue box rendering and backpack rendering using GPU quads and its own `draw_text_sub`.

**Why it is a problem:**
1. Two parallel implementations of the same UI: one in `ui/` (CPU buffer), one in `gpu/renderer.rs` (GPU quads). Changes to UI appearance must be applied in both places or they diverge silently.
2. The `ui` module's `draw_dialogue`, `draw_overworld_hud`, `draw_duel` functions are dead code in the GPU path. They exist only for the software fallback that is no longer used.
3. `ui/layout.rs` functions ARE used by both paths (GPU renderer imports `layout`), but `ui/panel.rs`, `ui/label.rs`, `ui/bar.rs` are CPU-buffer-only.

---

## CS-17 · `UiTheme` Conversion Logic Has Inconsistent Zero-Value Handling

**Location:** `ui/theme.rs:125-171`

**What happens:** `From<DialogueThemeFile> for UiTheme` checks each field individually (e.g., `if f.box_height > 0 { f.box_height } else { 60 }`). `merge_backpack` does the same for some fields but directly assigns others without checking (e.g., `self.backpack_panel_fill = f.panel_fill` — if the JSON has `[0,0,0]` it's applied; there's no way to distinguish "user set black" from "field was missing and defaulted to `[0,0,0]`").

**Why it is a problem:** `[f32; 3]` defaults to `[0.0, 0.0, 0.0]` (black), not "use the Rust default". If a theme JSON omits a color field, the panel becomes black instead of using the built-in theme color. The `#[serde(default)]` on the struct handles missing fields by zeroing them, which is then indistinguishable from "the author chose black."

---

## CS-18 · `load_theme` Uses Relative Path

**Location:** `ui/theme.rs:177`

**What happens:** `load_theme` hardcodes `"assets/ui/theme.json"` as a relative path, while every other loader uses `env!("CARGO_MANIFEST_DIR")` absolute paths (CS-11).

**Why it is a problem:** If the binary is run from a directory other than the project root, theme loading fails silently (returns default). All other asset loading uses the `CARGO_MANIFEST_DIR` pattern. Inconsistency.

---

## CS-19 · Layout Module Has Dead / Redundant Functions

**Location:** `ui/layout.rs:108-136`

**What happens:** `backpack_permanent_title_y` and `backpack_permanent_slot_y` duplicate the layout logic of `backpack_weapon_title_y` / `backpack_passive_title_y` but use a different calculation approach (dynamic `usable_count` parameter vs. fixed `BACKPACK_MAX_USABLE_SLOTS` constant). They are not called from the GPU renderer's backpack draw path.

**Why it is a problem:** Two layout calculation approaches for the same visual structure. One uses fixed slot reservation (used by GPU), one uses dynamic count (unused / dead). Adding a new backpack section requires deciding which approach to follow.

---

## CS-20 · `INTERACT_DISTANCE` and `OPPONENT_RANGE` Are Separate Constants With Same Value

**Location:** `state/overworld.rs:14` (`INTERACT_DISTANCE = 23.0`), `skills/apply.rs:10` (`OPPONENT_RANGE = 23.0`)

**What happens:** Two constants in two files, same value, same semantic meaning ("how close must the player be to interact with an NPC").

**Why it is a problem:** Changing one without the other creates inconsistency: the player can talk to an NPC but not shoot them, or vice versa. The duplication is not documented.
