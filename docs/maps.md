# Map assets (`assets/maps/`)

Each map lives in a folder named `{id}.map` (for example `mumhome.secondFloor.map`). The engine loads it with `map_loader::load_map("{id}")` (id without the `.map` suffix).

## Layout

| File | Purpose |
|------|---------|
| `map.json` | Grid size, tiles, collision semantics, optional spawn and tileset |
| `npc.json` | List of NPC instances (id + world position); character data comes from `assets/npc/` |
| `props.json` | Optional static props (buildings, large objects): `id` + world `position`; art in `assets/props/{id}.prop/` by convention. Omitted or empty = none. |
| `doors.json` | Optional list of map transitions (world-space rects → target map + spawn). Omitted or empty = no doors. |
| `scenes.json` | Optional list of proximity-based scene triggers (`MapSceneTrigger`). Omitted or empty = no scene triggers. |

## `map.json` fields

| Field | Type | Required | What it controls |
|-------|------|----------|------------------|
| `width` | number (integer) | yes | Number of columns in the tile grid. |
| `height` | number (integer) | yes | Number of rows in the tile grid. |
| `tiles` | array or object | yes | Define the tile grid. Supports three formats: **flat** row-major array (length `width * height`), **matrix** (array of rows), or **sparse** (object with `fill`, `perimeter`, and `rects`). See below. |
| `tile_size` | number | yes | Edge length of one tile in **world units** (same units as entity positions and movement). Used for collision bounds and drawing tile quads. |
| `player_start` | `[x, y]` | no | Player spawn in world units **only when the game calls `setup_world` with this map’s `player_start`**. In the current binary, that happens **once at cold start** in `main.rs` for the **initial** map id (e.g. `mumhome.secondFloor`). **Changing `player_start` on `mumhome.firstFloor` does not move you when you walk upstairs** from the second floor — that position comes from the **`spawn`** field on the **departure** map’s door (see `doors.json` below). If omitted, defaults to `[160, 160]`. |
| `tileset` | string | no | Base name of a PNG under `assets/tiles/{tileset}.png`. Optional `assets/tiles/{tileset}.json` can set `tile_size`. If the JSON is missing or `tile_size` does not divide the PNG, square sheets infer a size that yields a **4×4** grid when possible (e.g. 128×128 → 32px tiles). A wrong grid size makes `tileset_draw` indices point at the wrong pixels (garbled “Wang soup”). If the PNG is missing or load fails, the map renders from the palette only. **Map transitions:** the GPU re-uploads the tileset when the loaded sheet’s pixel data changes (`GpuRenderer::ensure_tileset`), so switching from an interior map to an exterior map (e.g. `farwest_interior` → `farwest_ground`) updates the texture correctly. |
| `tileset_tile_size` | number | no | **Per-tile pixel size inside the tileset PNG** (width and height of one cell in the sheet). When set, overrides `assets/tiles/{tileset}.json` — pin the grid here so the map file you edit is authoritative (e.g. `32` for PixelLab 128² wang sheets). On startup the binary prints the resolved grid to stderr (`map_loader::log_startup_tilemap_diagnostics`). |
| `tileset_draw` | object | no | When both a tileset is loaded **and** this object is present, each cell’s **logical** value is not drawn as a sheet index directly. Instead: logical `0` uses `tileset_draw.floor` as the sheet tile index; any non-zero logical value uses `tileset_draw.wall`, **unless** `wang_autotile` is true (see below). Collision uses only `tile_palette` (`walkable` per logical id). **Binary maps** (only ids `0` and `1` in `tiles`) **must** set `tileset_draw` when using a Wang interior sheet: otherwise `0` is treated as sheet tile 0 (a corner piece) and the whole floor looks like broken autotile. If `tileset_draw` is absent but a tileset is present, the GPU renderer tilemap pass uses palette solid fill for binary-only maps; if any tile id is `> 1`, stored numbers are used as sheet indices (clamped). |
| `tile_palette` | string | yes | Base name of a JSON file under `assets/tile_palettes/{tile_palette}.json`. For every **logical** tile id that appears in `tiles`, you should define `color` (RGB 0–1) and `walkable` (`true` / `false`). Solid-color rendering (no tileset) uses `color`; collision uses `walkable` only. Any id missing from the file blocks movement and draws magenta when filling. If the file is missing or invalid, `load_map` fails with `MapLoadError::MissingPalette`. |

### `tiles` and collision

- Collision is **only** from the palette: `walkable: true` means passable, `false` means blocking. A logical id not listed in the palette is treated as blocking.

### Sparse layouts

The **sparse** format is recommended for most maps as it is much more concise. It allows you to define a baseline and layer shapes on top.

```json
"tiles": {
  "fill": 0,
  "perimeter": 1,
  "rects": [
    { "id": 1, "x": 5, "y": 5, "w": 4, "h": 2 }
  ]
}
```

| Field | Type | What it controls |
|-------|------|------------------|
| `fill` | number | Optional (default `0`). The default tile ID for the entire grid. |
| `perimeter` | number | Optional. If set, automatically draws a 1-tile thick wall around the map boundary using this ID. |
| `rects` | array | Optional. A list of objects `{"id", "x", "y", "w", "h"}` to paint specific tile IDs over the grid. Useful for interior walls or pillars. |

### `tileset_draw` shape

```json
{ "floor": 6, "wall": 5, "wang_autotile": true }
```

- `floor` — index into the tileset grid for walkable cells (logical `0`). For PixelLab wang-16 sheets (`farwest_interior`, `farwest_ground`), index **6** is the all-floor wang tile (`base_tile_ids.lower` / `wang_0`). **Exterior** maps (e.g. `dustfall.junction`) use `farwest_ground` for desert plateau / street; **interiors** use `farwest_interior`.
- `wall` — index into the tileset grid for blocking cells when `wang_autotile` is false or omitted.
- `wang_autotile` (optional, default `false`) — when `true`, blocking cells choose a tile from a fixed 16-entry lookup for PixelLab-style 4×4 wang sets (straight edges, corners, thin segments). Implemented in `src/render/mod.rs` (`WANG16_SHEET_LUT`). Ignores `wall` for drawing.

### `tile_palette` file shape (`assets/tile_palettes/{id}.json`)

Keys are tile ids as strings; each value **must** include `color` and `walkable`:

```json
{
  "0": { "color": [0.35, 0.38, 0.4], "walkable": true },
  "1": { "color": [0.2, 0.18, 0.22], "walkable": false }
}
```

## `npc.json`

JSON **array** of objects. Each object is one NPC placed on this map.

| Field | Type | Required | What it controls |
|-------|------|----------|------------------|
| `id` | string | yes | Character id. Resolved to `assets/npc/{id}.npc/config.json` or legacy `assets/npc/{id}.npc.json` for `scale`, `color`, and optional `conversation_id`. See [npc.md](npc.md). |
| `position` | `[x, y]` | yes | World position for that NPC’s `Transform`. |

An **empty** array (`[]`) spawns **no** NPCs on that map. If `npc.json` is **missing**, the loader treats that as "no NPCs" and returns an empty list. If the file exists but JSON is invalid, loading fails with `MapLoadError`. If the array is non-empty but any referenced character config fails to load, `load_map` fails loudly instead of spawning fallback NPCs.

## `props.json`

Optional JSON **array**. If the file is **missing**, the map has no props. If **invalid JSON**, `load_map` fails with `MapLoadError`.

Each entry spawns one entity with `MapProp` + `Transform` + `Sprite` + `SpriteSheet` in `setup_world` (`src/ecs/world.rs`): **no** `Npc` component, so props never open dialogue. Preferred sheet path is `assets/props/{id}.prop/sheet.png` with optional `sheet.json` (`rows`, `cols`). CamelCase ids are preferred (for example `billyHouse` -> `assets/props/billyHouse.prop/`). Legacy plain folders and snake_case ids are still accepted during migration. Authoring: [ASSET_STYLE_GUIDE.md](../assets/ASSET_STYLE_GUIDE.md) — **Buildings**.

| Field | Type | Default | Role |
|-------|------|---------|------|
| `id` | string | (required) | Prop id. Preferred folder is `assets/props/{id}.prop/` (for example `billyHouse` -> `assets/props/billyHouse.prop/`). |
| `position` | `[x, y]` | (required) | World position (sprite **center**; same convention as entities). |
| `scale` | `[sx, sy]` | `[1, 1]` | Multiplier on sprite pixel size (after sheet frame size). |

**Collision:** Props are **visual only**. Blocking terrain must still be authored in `map.json` / `tile_palette` where the building should be impassable. **Opaque prop pixels are drawn over the tilemap** — yards, porches, and approaches the player should walk on need **transparent** pixels in `sheet.png` so underlying **walkable** tiles are visible (and the character is not covered by fake “ground” in the sprite).

**Building consistency:** Buildings may have different footprints and sprite sizes, but a town set should still share one art language. Keep projection, pixel density, outline treatment, shadow direction, and background transparency treatment consistent across neighboring building props. If one building uses a painted ground plate or a different camera angle, it will stand out immediately in-game even when collision is correct.

- **Wall rects** (sparse `tiles.rects` with a blocking palette id) should cover the prop sprite’s tile AABB on **north, east, and west**. An **extra southern** tile row may stay **walkable** when the sheet has a transparent porch or façade so the player can approach the door; misaligned or undersized rects let players walk **under** roof art.

**Interior example:** `mumhome.firstFloor.map` / `mumhome.secondFloor.map` use generic prop ids (`bed`, `table`, `dresser`, `stove`) in `props.json`; sheets live under `assets/props/{id}.prop/`. Use **top-down orthographic** art that matches `farwest_interior` (PixelLab’s `create_isometric_tile` MCP tool targets **isometric blocks** and tends to read as crates — prefer hand-drawn sprites, a top-down generator, or another pipeline for furniture-sized props).

**Example:** `dustfall.junction` lists `billyHouse`, town buildings (`clinic`, `sheriff`, `bank`, `saloon`, `emporium`), and smaller frontier props (`hitchPost`, `waterTrough`, `barrels`, `cactus`) in `props.json`; sheets live under `assets/props/{id}.prop/`. Place instances on **walkable** cells so the player is not stuck inside collision tiles.

## `doors.json` (map transitions)

Optional JSON **array**. If the file is **missing**, the map has no doors. If **invalid JSON**, `load_map` fails with `MapLoadError`.

A door is **not** a special tile in the renderer: collision stays palette-based. Use **floor** tiles (`walkable: true`) under the whole **`rect`** so the player can walk in. Align **`rect`** with those cells in **world units** (same as `player_start` / NPC positions). **Do not** let `map.json` **wall** rects (`id` blocking in the palette) overlap the door `rect` — e.g. extending a building footprint south into the same tile row as the door blocks movement and walk-through transitions.

**Door prop:** Optional field **`prop`** selects a door prop sheet under `assets/props/`. By convention, `"south"` resolves to `assets/props/south.door/` and `"southHeavy"` resolves to `assets/props/southHeavy.door/`. Missing / `"none"` means transition-only with no sprite. CamelCase ids are preferred, but older snake_case ids and legacy `visual` maps are still accepted during migration. `setup_world` derives frame size from the resolved sheet rather than hardcoding door variants. The quad uses **uniform** scale `min(width/frame_w, height/frame_h)` so the art is not stretched to the `rect`, is **horizontally centered** in the `rect`, and its **bottom** aligns with the rect bottom (same convention as props: position is the sprite center).

```json
[
  {
    "rect": [128, 256, 64, 32],
    "to_map": "mumhome.firstFloor",
    "spawn": [160, 112],
    "require_confirm": false,
    "prop": "southHeavy"
  }
]
```

| Field | Type | Default | Role |
|-------|------|---------|------|
| `rect` | `[min_x, min_y, width, height]` | (required) | Player **position** must lie inside this axis-aligned box. |
| `to_map` | string | (required) | Passed to `map_loader::load_map` (no `.map` suffix). |
| `spawn` | `[x, y]` | (required) | Player world position on the **destination** map after the transition. **Author this on the door that lives on the map you leave** (e.g. to tune where you appear on the second floor, edit `spawn` on the door inside `mumhome.firstFloor.map/doors.json`, not `player_start` in `mumhome.secondFloor.map/map.json`). |
| `require_confirm` | bool | `true` | If `false`, **walk-through**: transition on the **first frame** the player enters the rect (edge detection) — typical for doorways. If `true`, Space/Enter while inside the rect triggers the transition. A short cooldown after any **successful** transition avoids bounce. |
| `require_state` | string | omitted | Optional story gate: same condition syntax as dialogue branches (`flag:`, `path:`, `quest_active:`, `quest_complete:`). If set and the player’s `WorldState` does **not** satisfy it, the map does **not** change; use `deny_message`. Omitted = no gate. |
| `deny_message` | string | omitted | When `require_state` is set and fails, shown as a transient overworld banner ([docs/ui.md](ui.md)). Set this whenever you use `require_state`. |
| `prop` | string | omitted | Door prop id. `"south"` resolves to `assets/props/south.door/`; `"southHeavy"` resolves to `assets/props/southHeavy.door/`; `"none"` or missing means no door sprite. |

**Bidirectional links:** define a door on **each** map (e.g. stairs down on `mumhome.secondFloor`, stairs up on `mumhome.firstFloor`) with reciprocal `to_map` / `spawn` values. When the destination map has a walk-through door back (e.g. `mumhome.firstFloor` → `dustfall.junction`), tune **`spawn` on the departure door** so the player appears inside that destination door’s `rect` (often near its vertical center)—otherwise the exit can look far from the building entrance.

**Runtime:** `MapData` includes `doors: Vec<MapDoor>` (`config::MapDoor`, includes optional `prop`, optional `require_state` / `deny_message`) and `props: Vec<MapPropEntry>`. The game binary’s `App::update` (Overworld, backpack closed) calls `state::map_transition::poll_map_door_transition` with `WorldState`. On **transition**: `take_player_carryover` → `despawn_all_entities` → `load_map` → `setup_world` with door `spawn` and preserved **Backpack** + **Health** (`ecs/world.rs`). On **blocked** gate: no transition and no transition cooldown; the player can step away and retry. After the post-transition **cooldown** (`DOOR_TRANSITION_COOLDOWN_SECS`), overlap memory is seeded from the player’s current position so if the **`spawn` lies inside a walk-through door rect** on the destination map, the game does not immediately treat that as a new entry. Constants: `DOOR_TRANSITION_COOLDOWN_SECS`, `OVERWORLD_TOAST_DURATION_SECS` in `constants.rs`.

## `scenes.json` (scene triggers)

Optional JSON **array**. If the file is **missing**, the map has no scene triggers. If **invalid JSON**, `load_map` fails with `MapLoadError`.

Each entry defines a proximity-based trigger that auto-fires a data-driven scene when the player walks into range.

| Field | Type | Default | Role |
|-------|------|---------|------|
| `scene_id` | string | (required) | Scene definition id. Resolved to `assets/scenes/{scene_id}.scene.json`. |
| `trigger_position` | `[x, y]` | (required) | World position of the trigger center. |
| `trigger_radius` | number | (required) | Trigger fires when player distance ≤ this value (world units). |
| `require_not_flag` | string | omitted | If set, trigger only fires if this `WorldState` flag is **not** set. Use to make scenes once-only. |

Scenes are loaded on demand from `assets/scenes/{id}.scene.json` via `SceneCache` in `src/scene/`. A `SetFlag` step in the scene can gate it to play once only by writing a flag that `require_not_flag` checks. Missing scene file logs an error and returns to Overworld (no crash).

**Runtime:** `MapData` includes `scene_triggers: Vec<MapSceneTrigger>` (`config::MapSceneTrigger`). `update_overworld` checks player distance each frame (gated by `scene_trigger_cooldown`) and transitions to `AppState::Scene` when a trigger fires. `SCENE_TRIGGER_COOLDOWN_SECS` in `constants.rs` prevents re-entry immediately after a scene ends.

## Related

- NPC character files, dialogue ids, and runtime merge: [npc.md](npc.md).
- Rendering behavior with tilesets and wang autotile: [architecture.md](architecture.md), `src/render/mod.rs`.
- Loader implementation: `src/map_loader.rs`, `src/map.rs`, `src/config.rs` (`MapNpcEntry`, `MapPropEntry`, `MapDoor`, `MapSceneTrigger`).
- Overworld proximity / interact: `src/constants.rs` (`NPC_INTERACT_RANGE`), `src/state/overworld.rs`, `src/state/app.rs`.
- Doors: `src/state/map_transition.rs`, `src/main.rs` (`apply_map_transition`); door sprites: `ecs/world.rs` (`DoorMarker`). Props: `ecs/world.rs` (`MapProp`); sheets: `assets.rs` (`load_character_sheet`).
