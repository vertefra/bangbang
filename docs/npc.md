# NPC configuration

NPCs are **data-driven** in two places: **where** they appear (per map) and **how** they look and which conversation they use (per character id).

## Quick checklist (new NPC)

1. Add **`assets/npc/{id}.npc/config.json`** with `scale`, `color`, and optionally `conversation_id` (see below).
2. Add **`assets/dialogue/{conversation_id}.json`** if you want scripted dialogue. If `conversation_id` is omitted in config, it defaults to **`{id}`** (same string as the map entry’s `id`).
3. Optionally add **`assets/npc/{id}.npc/sheet.png`** (and **`sheet.json`** for grid size). The renderer uses `SpriteSheet.character_id` = map `id`; if the sheet is missing, the NPC draws as a solid `Sprite.color` quad. For **4-direction** PixelLab characters, stack rotation PNGs in **sheet row order** matching [`facing_sprite_row`](../src/render/mod.rs): **down → up → left → right** (PixelLab south, north, west, east), with **`sheet.json`** `rows` / `cols` set accordingly (e.g. `docSawbones`: 4×1).
4. Optionally add a **dialogue portrait** PNG: **`assets/npc/{id}.npc/portrait.png`**. If present, it is shown in the dialogue box when talking to that NPC. Prefer a **head-and-shoulders bust** (same idea as **`mom`**: **128×128** RGBA) so the left slot reads as a portrait, not a tiny full-body sprite. If there is no portrait file but a **`sheet.png`** exists, the dialogue UI falls back to the **idle “down”** frame of the walk sheet. If neither exists, dialogue text is unchanged (no portrait). (**`assets/characters/`** is reserved for the **player** sprite; see [ASSET_STYLE_GUIDE.md](../assets/ASSET_STYLE_GUIDE.md) — *File layout*.)
5. List the NPC in **`assets/maps/{map}.map/npc.json`**: `{ "id": "{id}", "position": [x, y] }` in world units (same as `player_start`). See [maps.md — `npc.json`](maps.md#npcjson).

## Map placement: `npc.json`

Each map’s `npc.json` is a JSON **array** of objects. Required fields per instance:

| Field | Type | Role |
|-------|------|------|
| `id` | string | Character id. Must match folder `assets/npc/{id}.npc/`. |
| `position` | `[x, y]` | World position; becomes the NPC entity’s `Transform.position`. |

**Not** defined here: `scale`, `color`, `conversation_id` — those come from the character config file.

Edge cases (missing file, failed loads) are described in [maps.md — `npc.json`](maps.md#npcjson).

## Character definition: `assets/npc/{id}.npc/config.json`

Parsed as [`CharacterNpcConfig`](../src/config.rs) (`serde` ignores unknown keys).

| Field | Type | Default | Role |
|-------|------|---------|------|
| `scale` | `[sx, sy]` | `[0.5, 0.5]` | `Transform.scale` on the NPC entity (affects drawn size with sheet or fallback quad). **World size** ≈ frame size × scale (see [`assets/ASSET_STYLE_GUIDE.md`](../assets/ASSET_STYLE_GUIDE.md) — *World scale*). The player uses **48×48** frames at scale **1.0**; match that (e.g. **48×48** art + `[1.0, 1.0]`, or **96×96** + `[0.5, 0.5]`). |
| `color` | `[r, g, b, a]` RGBA 0–1 | `[0.2, 0.6, 1.0, 1.0]` | `Sprite.color` (tint / solid fill when no character sheet). |
| `conversation_id` | string or omitted | NPC **`id`** from `npc.json` | Base name of `assets/dialogue/{conversation_id}.json`. Use when one script is shared by multiple map entries or ids. |

**Common mistake:** putting **`position`** in this file. It is **ignored**; only `npc.json` `position` is used when building [`NpcConfig`](../src/config.rs).

## Legacy layout: `assets/npc/{id}.npc.json`

The loader tries **`{id}.npc/config.json` first**, then falls back to **`{id}.npc.json`** in `assets/npc/`. A deprecation warning is logged for the legacy file. **New content** must use the folder layout only. The repo should not ship duplicate legacy files next to an existing `{id}.npc/` folder—remove **`{id}.npc.json`** after migrating fields into **`{id}.npc/config.json`**. See [ASSET_STYLE_GUIDE.md](../assets/ASSET_STYLE_GUIDE.md) — *File layout*.

## Runtime merge (`NpcConfig`)

[`map_loader::load_npcs_from_map`](../src/map_loader.rs) merges map + character data:

- `position` ← `npc.json`
- `scale`, `color` ← character config
- `conversation_id` ← character `conversation_id` or else map entry `id`

[`ecs::world::setup_world`](../src/ecs/world.rs) spawns each NPC with `Npc { id, conversation_id }`, `Transform`, `Sprite`, `SpriteSheet { character_id: id }`, `Facing`, and `Health`.

**NPCs** default to **10 / 10** health at spawn unless overridden. The **player** cold-start maximum is **5 / 5** (see [game.md](game.md) — Current state).

## Dialogue

- Files: **`assets/dialogue/{conversation_id}.json`**
- Shape: `start` (string) and `nodes` (object keyed by id). Each node may use `line` or `lines`, optional `next`, `branches`, `effects`. At load, `line` (if present) is stored as a single-element `lines` list (legacy: it overrides a `lines` array). Types live in `src/dialogue/tree.rs`.
- **Conversation Gating**: Conversations can define a top-level `"require_state": "condition"` and `"default_line": "Fallback text"`.
  - If the player's game state doesn't match `require_state`, the full conversation won't open; instead a one-shot fallback dialogue using `default_line` is shown.
  - Omit or set `require_state` to `null` to bypass gating.
- **Entry router:** A node with **no** `line`/`lines` but with `branches` can route the first real line by flag/path/quest (e.g. repeat NPC bark vs full intro). On dialogue open, [`dialogue::entry_point`](../src/dialogue/mod.rs) skips zero-line nodes so the player is not prompted on a blank page. Branch order matters: first matching `condition` wins; a branch with no `condition` matches always (use as default after specific conditions).
- **Conditions** (used in `branches` or `require_state`):
  - `flag:{name}` — true if `set_flag:{name}` was triggered previously.
  - `path:{name}` — true if `set_path:{name}` was triggered.
  - `quest_active:{id}` — true if `start_quest:{id}` was triggered and not yet completed.
  - `quest_complete:{id}` — true if `complete_quest:{id}` was triggered.
- **Effects** (triggered when a node is visited):
  - `set_flag:{name}` — sets a boolean flag for future reference.
  - `set_path:{name}` — sets a mutually exclusive path choice.
  - `start_quest:{id}` — marks a quest as active in the world state.
  - `complete_quest:{id}` — moves a quest from active to completed.
- If the file is missing, the game still opens dialogue using a **minimal fallback** conversation (placeholder text from app state, not from NPC JSON). Prefer shipping a real JSON file for each `conversation_id` you use.

## Interaction

- Proximity uses **`NPC_INTERACT_RANGE`** in `src/constants.rs` (also referenced from `src/state/overworld.rs`).
- When in range, overworld returns [`NpcInteraction`](../src/state/overworld.rs) with `npc_id` and `conversation_id` for `AppState::Dialogue`.

## Code reference

| Area | Location |
|------|----------|
| JSON types | `src/config.rs` — `MapNpcEntry`, `CharacterNpcConfig`, `NpcConfig` |
| Load + merge | `src/map_loader.rs` — `load_map`, `load_npcs_from_map`, `load_character_npc` |
| Portrait / sheets | `src/assets.rs` — `load_dialogue_portrait`, `get_dialogue_portrait_sheet`, `load_character_sheet` |
| Spawn | `src/ecs/world.rs` — `setup_world` |
| ECS tag | `src/ecs/components.rs` — `Npc` |
| Proximity | `src/state/overworld.rs` — `update`, `NpcInteraction` |
| Dialogue IO | `src/dialogue/loader.rs`, `src/dialogue/mod.rs` — `ConversationCache` |

## Related

- Map tiles and doors: [maps.md](maps.md)
- Dialogue behaviour in play: [game.md](game.md) (Dialogue bullet)
- High-level architecture: [architecture.md](architecture.md)
