# BangBang — UI Layer

## Overview

The UI is a **GPU-rendered** layer overlaid on top of the game world into the same wgpu `RenderPass`. It is organized as a dedicated `ui` module with **theme** and **layout** logic. All UI draws are executed as separate batched passes (e.g. `draw_ui_pass`) originating in `gpu/renderer.rs`.

There is no "software CPU canvas" or mutable pixel buffer iteration for the UI. Instead, logical game state prepares **UI data models** (e.g. `BackpackPanelLines`) natively within `App::update()`. These pre-computed string sets, geometries, and theme definitions are then fed immutably into the renderer.

---

## Layering and concepts

### 1. Draw order (GPU Passes)

1. **Tilemap** (`draw_tilemap_pass`): Background floors and grid structures.
2. **Entities** (`draw_entities_pass`): Players, NPCs, props with optional sprite sheets.
3. **UI panel quads** (`draw_ui_pass`): Dialogue panel fill/border, backpack panels (solid quads on the `white_over` batch).
4. **Dialogue portrait** (same frame, separate draw): If the player is in dialogue and the active NPC has a `portrait.png` or a character `sheet.png`, a textured quad is drawn **after** the dialogue panel and **before** dialogue text so the portrait sits on the panel with text to the right. Uses the same per-character texture bind groups as entities (`gpu/renderer.rs`).
5. **UI text** (`draw_ui_pass` + `font` batch): Dialogue lines and backpack labels via **vector text** (`fontdue` + Noto Sans, `gpu/text_atlas.rs`).
6. **Debug HUD** (`draw_debug_pass`): When the **`debug`** Cargo feature is on, `main` passes `Option<DebugOverlay>` (smoothed FPS plus preformatted lines for world position, tile indices, and palette fields). Drawn last in **black** using **Noto Sans Bold** (`layout_debug_text_quads` in `gpu/text_atlas.rs`), separate from regular UI’s Regular face.

Submission order defines stacking; dialogue text is intentionally drawn after the portrait.

### 2. Theme (`UiTheme`)

- **Role**: Holds all UI colors and dimensions (panel height, padding, border thickness, etc.). Single source of truth for look and layout geometry.
- **Location**: `src/ui/theme.rs`. Loaded once at startup via `ui::theme::load_theme()` from `assets/ui/theme.json`; on missing or parse error, it fails loudly via `Result` to prevent silent drift.
- **Format**: Theme colors are stored as `[f32; 3]` sRGB-style floats from `assets/ui/theme.json`, then converted during drawing to the renderer’s packed linear GPU color format. Sizes are **1× base pixels**. Panel/layout dimensions multiply by `render_settings.ui_scale`; **font em size** derives from `ui_scale × font_scale` (same config knob as before; clamped in code). Dialogue panel height is `dialogue.box_height` in `assets/ui/theme.json`. Optional dialogue portrait slot size and spacing: `dialogue.portrait_width`, `dialogue.portrait_height`, `dialogue.portrait_gap` (scaled by `ui_scale`). Dialogue and backpack strings use `fontdue` layout with optional **max width** so long lines wrap inside the panel (text max width shrinks when a portrait is shown).

### 3. Layout

- **Role**: Turns (screen size, theme, **`ui_scale`**) into **screen-space rects and positions** (left, top, right, bottom). 
- **Location**: `src/ui/layout.rs`. Examples include `dialogue_box_rect()`, `dialogue_portrait_rect()`, `dialogue_portrait_text_extra_left()`, `dialogue_text_pos(..., extra_left)` (non-zero `extra_left` when a portrait is visible), and `backpack_panel_rect()`. 

Keeping coordinate matrices and bounding geometry inside `layout.rs` actively removes "magic scaling numbers" from the `wgpu` rendering vertex setups.

---

## How to extend the UI

### Add a new screen or mode (e.g. pause menu, inventory)

1. **State Addition**: Create the new logical state (e.g. adding `Option<PauseMenuLines>` inside `App`).
2. **Data Model**: Prepare the GUI's textual layout during physics/logic steps in `App::update()`.
3. **Theme/Schema**: Inside `ui/theme.rs`, introduce sizing floats or colors into `UiTheme` and their companion `assets/ui/theme.json` maps.
4. **Layout Blocks**: Inside `layout.rs`, append a new mathematical function to evaluate dynamic pixel bounding boxes based on the UI Scale ratio.
5. **Renderer Pass**: Finally, inside `gpu/renderer.rs` inside the `draw_ui_pass()` method, call the layout rectangle bounds, then `push_ui_text()` → `UiFontAtlas::layout_text_quads()` → `push_quad` on the `font` batch (same textured pass as before). 
