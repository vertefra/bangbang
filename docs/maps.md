# Map assets (`assets/maps/`)

Each map lives in a folder named `{id}.map` (for example `intro.map`). The engine loads it with `map_loader::load_map("{id}")` (id without the `.map` suffix).

## Layout

| File | Purpose |
|------|---------|
| `map.json` | Grid size, tiles, collision semantics, optional spawn and tileset |
| `npc.json` | List of NPC instances (id + world position); character data comes from `assets/npc/` |

## `map.json` fields

| Field | Type | Required | What it controls |
|-------|------|----------|------------------|
| `width` | number (integer) | yes | Number of columns in the tile grid. |
| `height` | number (integer) | yes | Number of rows in the tile grid. |
| `tiles` | array | yes | Either **flat** row-major: one number per cell, length `width * height`, index `y * width + x`; or a **matrix**: array of exactly `height` rows, each row an array of exactly `width` numbers (row `0` = top of map). Invalid length or row count → loader uses fallback tilemap. |
| `tile_size` | number | yes | Edge length of one tile in **world units** (same units as entity positions and movement). Used for collision bounds and drawing tile quads. |
| `player_start` | `[x, y]` | no | Player spawn position in world units. If omitted, defaults to `[160, 160]`. |
| `tileset` | string | no | Base name of a PNG under `assets/tiles/{tileset}.png`. Optional `assets/tiles/{tileset}.json` can set `tile_size`. If the JSON is missing or `tile_size` does not divide the PNG, square sheets infer a size that yields a **4×4** grid when possible (e.g. 128×128 → 32px tiles). A wrong grid size makes `tileset_draw` indices point at the wrong pixels (garbled “Wang soup”). If the PNG is missing or load fails, the map renders from the palette only. |
| `tileset_tile_size` | number | no | **Per-tile pixel size inside the tileset PNG** (width and height of one cell in the sheet). When set, overrides `assets/tiles/{tileset}.json` — pin the grid here so the map file you edit is authoritative (e.g. `32` for PixelLab 128² wang sheets). On startup the binary prints the resolved grid to stderr (`map_loader::log_startup_tilemap_diagnostics`). |
| `tileset_draw` | object | no | When both a tileset is loaded **and** this object is present, each cell’s **logical** value is not drawn as a sheet index directly. Instead: logical `0` uses `tileset_draw.floor` as the sheet tile index; any non-zero logical value uses `tileset_draw.wall`, **unless** `wang_autotile` is true (see below). Collision uses only `tile_palette` (`walkable` per logical id). **Binary maps** (only ids `0` and `1` in `tiles`) **must** set `tileset_draw` when using a Wang interior sheet: otherwise `0` is treated as sheet tile 0 (a corner piece) and the whole floor looks like broken autotile. If `tileset_draw` is absent but a tileset is present, the GPU renderer tilemap pass uses palette solid fill for binary-only maps; if any tile id is `> 1`, stored numbers are used as sheet indices (clamped). |
| `tile_palette` | string | yes | Base name of a JSON file under `assets/tile_palettes/{tile_palette}.json`. For every **logical** tile id that appears in `tiles`, you should define `color` (RGB 0–1) and `walkable` (`true` / `false`). Solid-color rendering (no tileset) uses `color`; collision uses `walkable` only. Any id missing from the file blocks movement and draws magenta when filling. If the file is missing or invalid, the loader uses the internal fallback tilemap (see `map_loader::fallback_tilemap`). |

### `tiles` and collision

- Collision is **only** from the palette: `walkable: true` means passable, `false` means blocking. A logical id not listed in the palette is treated as blocking.

### `tileset_draw` shape

```json
{ "floor": 6, "wall": 5, "wang_autotile": true }
```

- `floor` — index into the tileset grid for walkable cells (logical `0`). For PixelLab wang-16 interior sheets (e.g. `farwest_interior`), index **6** is the all-floor wang tile (`base_tile_ids.lower` / `wang_0`).
- `wall` — index into the tileset grid for blocking cells when `wang_autotile` is false or omitted.
- `wang_autotile` (optional, default `false`) — when `true`, blocking cells choose a tile from a fixed 16-entry lookup for `farwest_interior`-style 4×4 wang sets (straight edges, corners, thin segments). Implemented in `src/gpu/wang.rs` (`WANG16_SHEET_LUT`). Ignores `wall` for drawing.

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
| `id` | string | yes | Character id. Resolved to `assets/npc/{id}.npc/config.json` or legacy `assets/npc/{id}.npc.json` for scale, color, dialogue fallback, and optional `conversation_id`. |
| `position` | `[x, y]` | yes | World position for that NPC’s `Transform`. |

If `npc.json` is missing, invalid, or no entries load successfully, the loader returns an explicit `MapLoadError` rather than crashing silently.

## Related

- NPC character fields: `docs/architecture.md` (Config / NPC definitions).
- Rendering behavior with tilesets: `docs/map_rendering_recommendations.md`.
- Loader implementation: `src/map_loader.rs`, `src/map.rs`, `src/config.rs` (`MapNpcEntry`).
