# BangBang — UI Layer

## Overview

The UI is a **CPU-rendered** layer drawn on top of the game world into the same framebuffer (no separate UI render target). It is organized as a dedicated `ui` module with **theme**, **layout**, and **reusable components**. There is **one draw entry point per game mode** (Overworld, Dialogue, Duel); the renderer chooses which to call based on `AppState`.

---

## Layering and concepts

### 1. Draw order

1. **World** (in `software::draw`): clear, tilemap, entities (Transform + Sprite/SpriteSheet).
2. **UI** (in `software::draw`, after world): calls one of `ui::draw_dialogue`, `ui::draw_overworld_hud`, or `ui::draw_duel` depending on state.

So UI is always on top. There is no z-order within the UI layer yet; elements are drawn in the order the entry function calls the components.

### 2. Theme (`UiTheme`)

- **Role**: Holds all UI colors and dimensions (panel height, padding, border thickness, etc.). Single source of truth for look and layout numbers.
- **Location**: `src/ui/theme.rs`. Loaded once at startup via `ui::load_theme()` from `assets/ui/theme.json`; on missing or parse error, `UiTheme::default()` is used.
- **Format**: Colors are `[r, g, b]` in 0.0–1.0. Sizes are in pixels (e.g. `dialogue_box_height`, `dialogue_padding_x`). The JSON uses a nested `"dialogue"` object; see `assets/ui/theme.json`.

Theme is **not** stored in the ECS. `App` owns the `UiTheme` and passes `&UiTheme` into `software::draw`, which passes it to the UI draw functions.

### 3. Layout

- **Role**: Turns (screen size, theme) into **screen-space rects and positions** (left, top, right, bottom or x, y). No drawing; only math.
- **Location**: `src/ui/layout.rs`. Functions like `dialogue_box_rect(w, h, theme)` and `dialogue_text_pos(...)` return coordinates. All layout constants come from `UiTheme`.

Keeping layout in one place avoids magic numbers in the draw code and makes it easy to adjust or add new screens.

### 4. Components (primitives)

Small, stateless draw functions that take buffer, dimensions, and explicit geometry/colors:

| Component | File | Purpose |
|-----------|------|---------|
| **Panel** | `ui/panel.rs` | Filled rectangle + optional top border strip. Used for dialogue box, future menus, duel panels. |
| **Label** | `ui/label.rs` | Single-line text at (x, y) using the shared 5×7 bitmap font (from `software`). |
| **Bar** | `ui/bar.rs` | Horizontal bar: background rect + filled portion by ratio in [0.0, 1.0]. For HP, stamina, etc. |

They use `crate::software::fill_rect` and `crate::software::draw_text` (and `to_u32` for theme colors). They do **not** know about theme or game state; callers pass in rects and u32 colors.

### 5. Per-mode entry points

- **`ui::draw_dialogue(buffer, w, h, theme, message)`** — Dialogue box at bottom + one line of text. Used when `AppState::Dialogue`.
- **`ui::draw_overworld_hud(buffer, w, h, theme)`** — Currently a no-op; later HP, money, etc.
- **`ui::draw_duel(buffer, w, h, theme)`** — Stub; later duel-specific panels, hand, stats.

`software::draw` decides which to call (right now it only calls `draw_dialogue` when `dialogue_message` is `Some`). No ECS or game types are used inside the UI module; it only receives buffer, size, theme, and simple data (e.g. `&str` for the message).

---

## How to extend the UI

### Add a new screen or mode (e.g. pause menu, inventory)

1. **Entry point**: In `src/ui/mod.rs`, add a new function, e.g. `pub fn draw_pause_menu(buffer, w, h, theme, options: &[&str], selected: usize)`.
2. **Theme**: In `UiTheme` (`ui/theme.rs`) add fields for colors/sizes (e.g. `pause_menu_bg`, `pause_menu_item_height`). In `assets/ui/theme.json` add a corresponding block (and in `UiThemeFile` a nested struct if you keep the nested JSON shape).
3. **Layout**: In `ui/layout.rs` add functions that return rects/positions for the new screen (e.g. `pause_menu_rect`, `pause_menu_item_y(index)`).
4. **Drawing**: In the new entry function, use `layout::*` for rects, `theme` for colors (convert with `crate::software::to_u32`), then call `draw_panel`, `draw_label`, (and `draw_bar` if needed). Compose from existing components where possible.
5. **Wiring**: In `software::draw` (or a dedicated draw path for that mode), call your new `ui::draw_*` when the appropriate state is active; pass theme and any context (e.g. menu options, selected index) from app/state.

### Add a new component (e.g. icon, list, progress ring)

1. Add a new file under `src/ui/`, e.g. `icon.rs` or `list.rs`.
2. Implement a `draw_*(buffer, w, h, ...)` function that uses `crate::software::fill_rect` and/or `crate::software::draw_text` (and later blit if you add sprite support). Prefer taking rects and u32 colors so the component stays independent of theme.
3. In `src/ui/mod.rs` add `mod icon;` (and optionally `pub use` if you want it public). Use the component from the relevant `draw_*` entry (e.g. `draw_duel`).

### Add or change theme fields

1. **Rust**: In `ui/theme.rs`, add fields to `UiTheme` and to `Default` (and to the file struct used for JSON, e.g. `DialogueThemeFile` or a new `PauseMenuThemeFile`). If you use a nested JSON block, add a struct and a field on `UiThemeFile`, and implement or extend `From<...> for UiTheme`.
2. **JSON**: In `assets/ui/theme.json` add the new keys (same structure as the Rust file struct). Existing keys can be changed for tuning; new keys will be used once the draw code reads them from `theme`.

### Add layout for a new region

- In `ui/layout.rs` add a function that takes `(screen_w, screen_h, &UiTheme)` (and any extra args, e.g. item count) and returns `(i32, i32, i32, i32)` for rects or `(i32, i32)` for points. Use theme fields for margins, sizes, and offsets. Call this from the relevant `draw_*` in `mod.rs`.

### Use the bar component (e.g. HP in HUD or Duel)

- In `ui::draw_overworld_hud` or `ui::draw_duel`, get the bar rect from a new layout function (e.g. `hud_hp_bar_rect(w, h, theme)`), get fill/empty colors from theme (and convert with `to_u32`), then call `bar::draw_bar(buffer, w, h, left, top, right, bottom, hp_ratio, fill_color, empty_color)`. You’ll need to pass HP (and max HP) into the draw function from app/state.

---

## File reference

| Path | Role |
|------|------|
| `src/ui/mod.rs` | Entry points: `draw_dialogue`, `draw_overworld_hud`, `draw_duel`; composes layout + panel + label. |
| `src/ui/theme.rs` | `UiTheme`, `load_theme()`, JSON deserialization. |
| `src/ui/layout.rs` | Screen-space rect and position helpers from theme + dimensions. |
| `src/ui/panel.rs` | `draw_panel` (fill + top border). |
| `src/ui/label.rs` | `draw_label` (single-line text). |
| `src/ui/bar.rs` | `draw_bar` (horizontal ratio bar). |
| `assets/ui/theme.json` | Data-driven colors and sizes; optional layout values. |

The shared font and `fill_rect`/`draw_text` live in `src/software.rs` and are used by the UI via `pub(crate)`.
