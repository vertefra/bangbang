---
name: fix-player-door-position
description: Diagnoses and fixes BangBang map door player spawn and transition issues (wrong position, bounce-back into building). Covers doors.json vs map.json, which file to edit for A→B transitions, coordinate conventions, and overlap with src/state/map_transition.rs. Use when the user reports door spawns, exits, mumhome, dustfall.junction, stairs, or map transitions.
---

# Fix player / door position

## When this applies

- Player appears in the wrong place **after** using a door or stairs.
- Player is **pulled back inside** shortly after exiting (walk-through door re-firing).
- Uncertainty about **`spawn`** vs **`player_start`** or which map file to edit.

## Read first

Load repository **`docs/maps.md`** (doors + `map.json` rules). For deep JSON field reference and coordinate notes, read [reference.md](reference.md) in this skill folder.

## Decision: what to change

| Symptom | Edit |
|--------|------|
| Wrong position on map **B** after leaving map **A** through a door | **`assets/maps/{A}.map/doors.json`** — the door whose `to_map` is **B**. Field: **`spawn`** (destination world position). |
| Wrong position on **first load** / initial map only | **`assets/maps/{initial}.map/map.json`** — **`player_start`**. Ignored when arriving via a door. |
| Transition never fires or wrong area for overlap | **`doors.json`** on **that** map — **`rect`** `[min_x, min_y, width, height]` in world units; player **center** must lie inside to trigger. |
| Door blocked by story / wrong message | Same door row — **`require_state`**, **`deny_message`**. |
| Bounce-back after exit when standing still on porch | Runtime: **`src/state/map_transition.rs`** — cooldown end must seed **`prev_door_overlap`** from current position (see reference). If logic is correct, reduce overlap by moving **`spawn`** slightly **outside** the destination walk-through **`rect`** (e.g. just south of it on the street). |

## Coordinate sanity (quick)

- Units are **world pixels**; **`tile_size`** is in **`map.json`** (often 32).
- **`spawn`** / **`player_start`** match the player **transform** (sprite **center** in rendering).
- Aligning with a building: use **`props.json`** on the destination map for prop **`position`** (center) + sprite size to reason about door steps vs **`rect`**.

## After edits

- Run `cargo test` (map loader tests cover several maps).
- Update **`docs/maps.md`** if authoring rules or runtime behavior changed.
