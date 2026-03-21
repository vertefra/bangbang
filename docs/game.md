# BangBang — Game Design & Roadmap

## Current state

- **Display:** `assets/config.json` sets **`render_scale`** (world zoom), **`ui_scale`** (bitmap font and UI panel sizes), and default **window** width/height (loaded at startup via `render_settings::load()`; missing/invalid file fails loudly). See [docs/architecture.md](architecture.md).
- **Maps**: First map is **intro** (`assets/maps/intro.map/`): `map.json` (tilemap, spawn, optional tileset) and `npc.json` (array of `{ "id", "position" }`, merged with `assets/npc/{id}.npc/…`). Loaded via `map_loader::load_map("intro")`; `setup_world(world, &map_data)` spawns player and NPCs. See [docs/maps.md](maps.md) for every JSON field.
- **Overworld**: Movement (WASD/arrows), tilemap collision, camera follows player. Player has `Facing` (direction) and `AnimationState` (idle/walk, frame index) for future sprite sheets; overworld updates them from input. All entities with Transform + Sprite are drawn (currently as colored rects).
- **Dialogue**: Dedicated `dialogue` module (`src/dialogue/`): loads conversations from `assets/dialogue/{conversation_id}.json` (nodes, branches, conditions, effects). Player within range of an NPC triggers `AppState::Dialogue { npc_id, conversation_id, node_id, line_index }`; dialogue module resolves current line and advance (multi-line paging, branch conditions on `StoryState` flags/path, effects like `set_flag`/`set_path`). Space/Enter advances within node or to next node, or closes and returns to Overworld. NPC config has optional `conversation_id` (defaults to NPC id); an explicit error logs and returns without crashing if no conversation file exists.
- **Debug / FPS (developers):** Building with the Cargo feature **`debug`** (`cargo run --features debug`) shows a smoothed **FPS** counter in the top-left. It is compile-time only (no in-game toggle). Details: [docs/architecture.md — Build features](architecture.md#debug-fps-overlay).
- **Skills (data-driven):** `assets/skills/{id}.json` defines `category` / `subcategory` and `effects` (`deal_damage` | `heal`, `target` `caster` | `opponent`, `amount`). `SkillRegistry::load_from_dir` dynamically loads JSON defs. `seed_demo_backpack` gives the player a permanent **Sidearm** (equipped weapon) and usable **Beer** (charges from JSON). **B** opens the backpack (Overworld); **Tab** / **Shift+Tab** cycle equipped weapon; **1** fires the equipped weapon vs nearest NPC in range; **2** uses the first usable. See [docs/skills.md](skills.md). HP is on `Health` for player and NPCs.

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

**Scaling: more characters and objects.** Maps and NPCs are data-driven: `assets/maps/{id}.map/map.json` and `npc.json` (array of refs to `assets/npc/{id}.npc.json`); `setup_world(world, &map_data)` spawns player and NPCs. Every entity with `Transform` + `Sprite` is drawn. Add more NPCs by adding `assets/npc/{id}.npc.json` and referencing them in a map's `npc.json`, or add new maps. For draw order, add a `Layer`/`ZOrder` component and sort in the renderer. Use components (`Player`, `Npc`, `Prop`, `Facing`, `AnimationState`, etc.) to distinguish kinds; the renderer stays generic.

---

## Plan

Phases from current state to a **playable game** (one loop: explore → talk/duel → consequences → repeat).

| Phase | Name | Status | Deliverables |
|-------|------|--------|--------------|
| **0** | Boilerplate & ECS init | **Done** | GPU renderer (wgpu), winit window/event loop, hecs World, AppState stub (Overworld / Dialogue / Duel). |
| **1** | Movement & tilemap | **Done** | Player entity, input handling, movement system. Tilemap (single map), camera following player. Collision so the player can’t walk through walls. Overworld traversable; rasterization on GPU (integrated graphics typical). |
| **2** | Duel engine | Next | Effect trait and effect list execution. Gun Power stat; damage and healing scale from it. Turn order, hand of “cards” (skills), play skill → apply effects. Win/lose outcome. `AppState::Duel` drives one duel from start to result. |
| **3** | Skills & inventory | — | Data-driven skill definitions (config/JSON): list of effects per skill. Permanent vs Usable; Usable has charges. Inventory resource or component; gain/lose skills. Duel and overworld can consume Usable skills. |
| **4** | Story & dialogue | Partial | Done: `dialogue` module, `assets/dialogue/{id}.json` (nodes, `line`/`lines`, `next`, `branches` with conditions, `effects`). Multi-line paging, `StoryState` for conditions and effects. Next: choices UI, more triggers. |
| **5** | Consequence loop | — | On duel loss: trigger Emergency Event. Apply medical fees (Money resource), remove some Usable skills, optionally apply Skill Fatigue (disable Permanent skills for N turns or until rest). No game over; return to Overworld. Persist or reset Money/Inventory as desired for MVP. |
| **6** | Playable MVP | — | One small overworld map, 1–2 NPCs, one dialogue path, one duel (win/lose both paths), consequence on loss. Full loop: start → walk → talk or duel → if lost, consequences → repeat. Polish: basic UI (HP, money, dialogue box), sound/feedback optional. |

After Phase 6 the game is **playable** end-to-end. Later expansion (more characters, conversations, weapons, world, mechanics) uses the same architecture as in the Game Expansion section above.

---

## Todo (tracking)

**Defer** (do when needed, not before):
- **Layer/ZOrder**: Add a `Layer` or `ZOrder` component and sort by it in the renderer when you have overlapping entities (e.g. props, shadows) that need explicit draw order.
- **Map switching**: Add in-game transitions (e.g. doors, triggers) that call `map_loader::load_map(id)` and re-run `setup_world`; clear/replace world and tilemap as needed. Not required until multiple areas exist.

**When starting Phase 2** (Duel engine):
- **Duel entry**: Add a way to transition into `AppState::Duel` (e.g. NPC interaction or dialogue choice “Fight”). From Overworld or Dialogue, set `*self = AppState::Duel` (and later pass duel context if needed).
- **Duel draw**: In main’s redraw, branch on `AppState`: when `Duel`, call a duel-specific draw (e.g. `software::draw_duel(...)`) instead of overworld tilemap + entities. Keep existing `software::draw` for Overworld/Dialogue.
