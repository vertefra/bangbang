# Reference: JSON objects and door/spawn tweaking

BangBang paths: `assets/maps/{map_id}.map/` — map id has **dots**, folder name is `{map_id}.map`.

---

## 1. `doors.json` (per map)

**Path:** `assets/maps/{map_id}.map/doors.json`  
**Shape:** JSON **array** of door objects. Missing file ⇒ no doors on that map.

### Door object fields

| Field | Type | Role |
|-------|------|------|
| **`rect`** | `[f32; 4]` | **`[min_x, min_y, width, height]`** in **world units**. Player **position** (sprite center) must be inside for overlap / confirm. |
| **`to_map`** | string | Destination map id **without** `.map` (e.g. `dustfall.junction`). |
| **`spawn`** | `[f32; 2]` | **`[x, y]`** where the player appears on **`to_map`** after this transition. **Authored on the map you leave** (departure map), not on the destination. |
| **`require_confirm`** | bool | `false` = walk-through (fires on **edge** into rect); `true` = need confirm while inside. Default in schema often `true` if omitted — check `src/config.rs` / `docs/maps.md`. |
| **`prop`** | string optional | Door sprite id (`"south"`, `"southHeavy"`, `"none"`, etc.). |
| **`require_state`** | string optional | Story gate (`flag:…`, `quest_active:…`, …). |
| **`deny_message`** | string optional | Banner when gate fails. |

### Which door to tweak for A → B

- **Landing position on B:** edit **`spawn`** on the row in **`{A}.map/doors.json`** where **`to_map` == `"B"`** (same string as `load_map` uses).
- **Trigger volume on B** (stepping into building): edit **`rect`** on **`{B}.map/doors.json`** for the door back to A (or whichever door it is).

### Walk-through + cooldown (bounce-back)

- **`src/state/map_transition.rs`** — `poll_map_door_transition`: after **`DOOR_TRANSITION_COOLDOWN_SECS`**, overlap memory must reflect **current** door index if the player is still inside a rect; otherwise the next frame looks like a **new entry** and re-triggers.
- If **`spawn`** on the departure door places the player **inside** the destination’s walk-through **`rect`**, logic above prevents instant re-entry; optionally move **`spawn`** just **outside** that **`rect`** (e.g. past `min_y + height` if the door band is horizontal) for clearer “on the street” placement.

---

## 2. `map.json` (per map)

**Path:** `assets/maps/{map_id}.map/map.json`

| Field | Relevance to doors |
|-------|---------------------|
| **`tile_size`** | Converts world coords ↔ tile indices for debugging / layout. |
| **`player_start`** | Player spawn **only** when the game starts on this map **without** arriving via a door (e.g. cold start from `game.json` **`start_map`**). **Does not** set position when entering this map through **`doors.json` **`spawn`**. |
| **`tiles` / collision** | Walkable vs blocking — door **`rect`** should sit on walkable cells; don’t overlap blocking rects with the transition zone. |

---

## 3. `props.json` (optional, per map)

**Path:** `assets/maps/{map_id}.map/props.json`

| Field | Role |
|-------|------|
| **`id`** | Prop asset (e.g. `billyHouse`). |
| **`position`** | **`[x, y]`** world — prop placement (renderer uses center with sprite size for quads). |
| **`scale`** | Scale factors. |

Use props + **`doors.json`** **`rect`** on the same map to align “porch” vs street **`spawn`** values.

---

## 4. `game.json` (bootstrap)

**Path:** `assets/game.json`

| Field | Role |
|-------|------|
| **`start_map`** | Initial map id — only affects which **`player_start`** / first **`setup_world`** runs at boot. |

---

## 5. Code touchpoints (not JSON)

| File | Why |
|------|-----|
| **`src/state/map_transition.rs`** | Door polling, cooldown, **`prev_door_overlap`**, walk-through edge detection. |
| **`src/config.rs`** | **`MapDoor`** struct — serde defaults for **`require_confirm`**, optional fields. |
| **`src/ecs/world.rs`** | **`setup_world(..., player_spawn, ...)`** applies **`spawn`** / **`player_start`**. |
| **`src/main.rs`** | **`apply_map_transition`**, loads map and calls **`setup_world`** with **`door.spawn`**. |
| **`src/constants.rs`** | **`DOOR_TRANSITION_COOLDOWN_SECS`**. |

---

## 6. Example (mum home ↔ junction)

- Exit **mumhome.firstFloor** → **dustfall.junction**: tweak **`spawn`** in **`mumhome.firstFloor.map/doors.json`** on the door with **`to_map`: `dustfall.junction`**.
- Enter house from outside: **`rect`** and return **`spawn`** live in **`dustfall.junction.map/doors.json`** (door to **`mumhome.firstFloor`**).

---

## Canonical doc

Repository **`docs/maps.md`** is authoritative for format and runtime; keep this reference aligned when behavior changes.
