# BangBang — Game Design & Roadmap

## Current state

- **Display:** `assets/config.json` sets **`render_scale`** (world zoom), **`ui_scale`** (UI panel layout and theme pixel steps), optional **`font_scale`** (multiplier on UI text em size; default `1.0`), and default **window** width/height (loaded at startup via `render_settings::load()`; missing/invalid file fails loudly). See [docs/architecture.md](architecture.md).
- **Bootstrap:** `assets/game.json` sets the **start map** id (default `mumhome.secondFloor`), optional **`seed_demo_backpack`**, and **window title**. Loaded via `config::GameConfig::load()` before the first map load.
- **Maps**: The initial map comes from **`game.json`** `start_map` (default **mumhome.secondFloor**) (`assets/maps/mumhome.secondFloor.map/`): `map.json`, optional `npc.json`, optional `doors.json`. **mumhome.firstFloor** (mom downstairs) links to the exterior town map **dustfall.junction** (`assets/maps/dustfall.junction.map/`, `farwest_ground` tileset, 64×48 tiles). **dustfall.junction** is the full Dustfall Junction town (per `story/MAP.md`) with 6 building props placed along the north side of a central main street: `billyHouse`, `clinic`, `sheriff`, `bank`, `saloon`, `emporium`. **Doc Sawbones** (`docSawbones` in `npc.json`) stands on the junction near the **clinic** prop; `assets/dialogue/docSawbones.json` is tutorial dialogue that foreshadows respawn and duel-loss penalties—those systems are **not** fully implemented yet. Buildings are visual props only (no interior maps yet); collision footprints are defined in `map.json` wall rects. Optional **`props.json`** spawns static sprites from `assets/props/{id}.prop/` by convention (e.g. `billyHouse` -> `assets/props/billyHouse.prop/`). `load_map` fills `MapData`; `setup_world` spawns player, NPCs, and props. **Doors:** each entry may set optional **`prop`** (`"south"` resolves to `assets/props/south.door/`; `"southHeavy"` resolves to `assets/props/southHeavy.door/`; `"none"` or missing means no overlay). Walking into a door rect and pressing Space/Enter (or walking in, if `require_confirm` is false) either transitions (new tilemap/tileset, world cleared and rebuilt, **Backpack** and **Health** kept) or, if the door defines **`require_state`** and story state does not match, shows a short on-screen message and stays on the map (e.g. leaving the house before talking to Mom). See [docs/maps.md](maps.md).
- **Overworld**: Movement (WASD/arrows), tilemap collision, camera follows player. Player has `Facing` (direction) and `AnimationState` (idle/walk, frame index) for future sprite sheets; overworld updates them from input. All entities with Transform + Sprite are drawn (currently as colored rects).
- **Dialogue**: Dedicated `dialogue` module (`src/dialogue/`): loads conversations from `assets/dialogue/{conversation_id}.json` (nodes, branches, conditions, effects). Player within range of an NPC triggers `AppState::Dialogue` (holds `npc_id`, `conversation_id`, `node_id`, `line_index`, plus per-line typewriter state). The current line streams in character-by-character at `DIALOGUE_CHARS_PER_SEC` (`src/constants.rs`). Space/Enter while still typing reveals the rest of that line; on a fully shown line, Space/Enter advances to the next line in the node, then to the next node, or closes and returns to Overworld when the conversation ends (multi-line paging, branch conditions on `WorldState` flags/quests/path, effects like `start_quest`/`set_path`). Entire conversations can also be gated behind a `require_state` condition, with a `default_line` fallback shown otherwise. Draw path uses `dialogue_display_text` (visible prefix); `dialogue_message` is the full current line. Optional NPC `conversation_id` defaults to the map entry’s NPC id; if the JSON file is missing, a tiny placeholder conversation is used. Authoring: [docs/npc.md](npc.md).
- **Debug HUD (developers):** Building with the Cargo feature **`debug`** (`cargo run --features debug`) shows a top-left overlay: smoothed **FPS**, player **world x/y**, **tile grid** coordinates (with `OOB` when outside the map), and **tile properties** from the palette (logical id, walkable, blocking, RGB). It also draws **red borders** around each world sprite AABB and **black coordinate labels** (`Transform.position`) next to them. Strings for the HUD are built in `main.rs` as `gpu::DebugOverlay`; the renderer draws overlays and entity debug. Compile-time only (no in-game toggle). See [docs/architecture.md — Debug overlay](architecture.md#debug-overlay) and [docs/ui.md](ui.md).
- **Skills (data-driven):** `assets/skills/{id}.json` defines `category` / `subcategory` and `effects` (`deal_damage` | `heal`, `target` `caster` | `opponent`, `amount`). `SkillRegistry::load_builtins` dynamically loads JSON defs (fails if none). When `game.json` has `seed_demo_backpack: true`, seeding gives the player a permanent **Sidearm** (equipped weapon) and usable **Beer** (charges from JSON). **B** opens the backpack (Overworld); **Tab** / **Shift+Tab** cycle equipped weapon; **1** fires the equipped weapon vs nearest NPC in range; **2** uses the first usable. See [docs/skills.md](skills.md). HP is on `Health` for player and NPCs. **Cold start**, the player begins at **5/5** LP and the HUD shows current/max health.

## Game Idea (MVP)

**BangBang** is a 2D top-down adventure with turn-based duels and branching story. The playable MVP is:

- **Overworld**: Explore a small world (Pokémon / Stardew Valley style). Move, discover locations, meet characters, and trigger duels or dialogue.
- **Duels**: Turn-based combat inspired by *Bang!* the card game. No distance mechanic; **Gun Power** scales skill effectiveness and damage. Use **Permanent Skills** (e.g. Take Cover, Whistle for Horse) and **Usable Skills** (e.g. Beer for HP, Gatling) with limited charges.
- **Story**: Progression is driven by three archetypes — **The Sheriff**, **The Bandit**, **The Renegade**. Choices and outcomes branch via dialogue and faction standing.
- **Consequences**: Losing a duel is not game over. It triggers an **Emergency Event**: medical fees (lose Money), inventory loss (lose some Usable Skills), and possibly **Skill Fatigue** (temporary disable of Permanent Skills). The player continues and can recover.

Skills are **data-driven**: each skill is a list of effects (ModifyDefense, Heal, BuffPower, etc.), so new skills can be added via config without code changes.

---

## Game Expansion

Expansion is designed to slot into the existing architecture without rewrites.

| Expansion | What to add | Where it fits |
|-----------|--------------|----------------|
| **Characters** | New NPCs, portraits, names, factions. | ECS: new components (e.g. `Npc`, `Faction`, `Portrait`). State: same `Overworld` / `Dialogue` / `Duel`; dialogue and duel systems resolve by entity ID. Data: JSON or asset config per character. |
| **Conversations** | More dialogue trees, branches, conditions, rewards. | `dialogue` module loads `assets/dialogue/{id}.json`; `AppState::Dialogue` holds `conversation_id`, `node_id`, `line_index`. Add more JSON files and reference via NPC `conversation_id`. Conditions (flag/path) and effects already supported. |
| **Weapons / skills** | New guns, items, permanent skills, effect types. | Data-driven: new rows in skill/weapon configs and new effect types (e.g. `DealDamage`, `DrawCard`). ECS: inventory and duel state as components or resources. Duel engine applies effect list; no new gameplay code per weapon. |
| **World** | New maps, areas, tiles, collision, NPC positions. | ECS: tilemap as component or resource; `Transform` + collision layers. `GpuRenderer::draw_frame` draws tilemap + entities. Load from Tiled/JSON + asset paths. |
| **Mechanics** | Minigames, stealth, reputation, day/night. | New components (e.g. `Reputation`, `TimeOfDay`) and systems in `ecs`. Optional new `AppState` variants (e.g. `Minigame`) or sub-states inside Overworld. State machine stays the same; only new branches and systems. |

Principles:

- **ECS**: All game objects are entities; new behaviour = new components + systems. No “god objects”.
- **State machine**: All modes go through `AppState`; new modes = new enum variants and their `update` (and drawing in main via `software` when in Overworld).
- **Data-driven**: Content (dialogue, skills, maps) lives in config/JSON; code defines *kinds* of content (effects, branches, tile layers).

**Scaling: more characters and objects.** Maps and NPCs are data-driven: `assets/maps/{id}.map/map.json` and `npc.json` (array of `{ id, position }` pointing at `assets/npc/{id}.npc/config.json` or legacy `{id}.npc.json`); `setup_world(world, &map_data)` spawns player and NPCs. Every entity with `Transform` + `Sprite` is drawn. See [docs/npc.md](npc.md). For draw order, add a `Layer`/`ZOrder` component and sort in the renderer. Use components (`Player`, `Npc`, `Prop`, `Facing`, `AnimationState`, etc.) to distinguish kinds; the renderer stays generic.

---

## Plan

Phases from current state to a **playable game** (one loop: explore → talk/duel → consequences → repeat).

| Phase | Name | Status | Deliverables |
|-------|------|--------|--------------|
| **0** | Boilerplate & ECS init | **Done** | GPU renderer (wgpu), winit window/event loop, hecs World, AppState stub (Overworld / Dialogue / Duel). |
| **1** | Movement & tilemap | **Done** | Player entity, input handling, movement system. Tilemap (single map), camera following player. Collision so the player can’t walk through walls. Overworld traversable; rasterization on GPU (integrated graphics typical). |
| **2** | Duel engine | Next | Effect trait and effect list execution. Gun Power stat; damage and healing scale from it. Turn order, hand of “cards” (skills), play skill → apply effects. Win/lose outcome. `AppState::Duel` drives one duel from start to result. |
| **3** | Skills & inventory | — | Data-driven skill definitions (config/JSON): list of effects per skill. Permanent vs Usable; Usable has charges. Inventory resource or component; gain/lose skills. Duel and overworld can consume Usable skills. |
| **4** | Story & dialogue | Partial | Done: `dialogue` module, `assets/dialogue/{id}.json` (nodes, `line`/`lines`, `next`, `branches` with conditions, `effects`). Multi-line paging, `WorldState` for conditions and effects. Next: choices UI, more triggers. |
| **5** | Consequence loop | — | On duel loss: trigger Emergency Event. Apply medical fees (Money resource), remove some Usable skills, optionally apply Skill Fatigue (disable Permanent skills for N turns or until rest). No game over; return to Overworld. Persist or reset Money/Inventory as desired for MVP. |
| **6** | Playable MVP | — | One small overworld map, 1–2 NPCs, one dialogue path, one duel (win/lose both paths), consequence on loss. Full loop: start → walk → talk or duel → if lost, consequences → repeat. Polish: basic UI (HP, money, dialogue box), sound/feedback optional. |

After Phase 6 the game is **playable** end-to-end. Later expansion (more characters, conversations, weapons, world, mechanics) uses the same architecture as in the Game Expansion section above.

---

## Todo (tracking)

**Defer** (do when needed, not before):
- **Layer/ZOrder**: Add a `Layer` or `ZOrder` component and sort by it in the renderer when you have overlapping entities (e.g. props, shadows) that need explicit draw order.

**Done (baseline):**
- **Map switching**: `doors.json` + `apply_map_transition` in `main.rs` (preserve inventory/HP). Extend with more maps and rects as needed.

**When starting Phase 2** (Duel engine):
- **Duel entry**: Add a way to transition into `AppState::Duel` (e.g. NPC interaction or dialogue choice “Fight”). From Overworld or Dialogue, set `*self = AppState::Duel` (and later pass duel context if needed).
- **Duel draw**: In main’s redraw, branch on `AppState`: when `Duel`, call a duel-specific GPU draw path instead of the current overworld tilemap + entities frame path.
