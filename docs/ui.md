# BangBang — UI Layer

## Overview

The UI is a **GPU-rendered** layer overlaid on top of the game world into the same wgpu `RenderPass`. It is organized as a dedicated `ui` module with **theme** and **layout** logic. All UI draws are executed as separate batched passes (e.g. `draw_ui_pass`) originating in `gpu/renderer.rs`.

There is no "software CPU canvas" or mutable pixel buffer iteration for the UI. Instead, logical game state prepares **UI data models** (e.g. `BackpackPanelLines`) natively within `App::update()`. These pre-computed string sets, geometries, and theme definitions are then fed immutably into the renderer.

---

## Layering and concepts

### 1. Draw order (GPU Passes)

1. **Tilemap** (`draw_tilemap_pass`): Background floors and grid structures.
2. **Entities** (`draw_entities_pass`): Players, NPCs, props with optional sprite sheets.
3. **UI** (`draw_ui_pass`): Dialogue boxes, backpack panels, hotbars. Built primarily from solid color quads and the font atlas.
4. **Debug FPS** (`draw_debug_pass`): Optional diagnostics overlay (e.g. `FPS:60`) drawn last.

There is no z-order within the UI layer itself yet; elements are drawn natively from bottom to top via GPU submission ordering.

### 2. Theme (`UiTheme`)

- **Role**: Holds all UI colors and dimensions (panel height, padding, border thickness, etc.). Single source of truth for look and layout geometry.
- **Location**: `src/ui/theme.rs`. Loaded once at startup via `ui::theme::load_theme()` from `assets/ui/theme.json`; on missing or parse error, it fails loudly via `Result` to prevent silent drift.
- **Format**: Colors are strictly evaluated into `[f32; 3]` linear floats up-front. Sizes are **1× base pixels**. Dimensions multiply dynamically by `render_settings.ui_scale` during draw evaluation, enabling pixel-perfect upscaling of the 5x7 bitmap font.

### 3. Layout

- **Role**: Turns (screen size, theme, **`ui_scale`**) into **screen-space rects and positions** (left, top, right, bottom). 
- **Location**: `src/ui/layout.rs`. Examples include `dialogue_box_rect()` and `backpack_panel_rect()`. 

Keeping coordinate matrices and bounding geometry inside `layout.rs` actively removes "magic scaling numbers" from the `wgpu` rendering vertex setups.

---

## How to extend the UI

### Add a new screen or mode (e.g. pause menu, inventory)

1. **State Addition**: Create the new logical state (e.g. adding `Option<PauseMenuLines>` inside `App`).
2. **Data Model**: Prepare the GUI's textual layout during physics/logic steps in `App::update()`.
3. **Theme/Schema**: Inside `ui/theme.rs`, introduce sizing floats or colors into `UiTheme` and their companion `assets/ui/theme.json` maps.
4. **Layout Blocks**: Inside `layout.rs`, append a new mathematical function to evaluate dynamic pixel bounding boxes based on the UI Scale ratio.
5. **Renderer Pass**: Finally, inside `gpu/renderer.rs` inside the `draw_ui_pass()` method, call the layout rectangle bounds, then append `push_quad` commands natively against the `white_over` or `font` UI batches. 
