# plan.door-world-gate.md — Data-driven door gates (WorldState)

**Goal:** Reusable map transition gates: a door may require a story condition (same string syntax as dialogue branches: `flag:`, `path:`, `quest_active:`, `quest_complete:`). If the player triggers that door while the condition is false, no transition; show a short on-screen message instead (e.g. first-floor exit until Billy has finished Mom’s intro).

**Decisions:** Extend `MapDoor` in `config.rs` with optional `require_state` and `deny_message`. Reuse `dialogue::tree::condition_matches` via `dialogue::world_state_satisfies`; refactor `state_satisfied` to delegate to it (single trim/empty policy). Extend `poll_map_door_transition` to accept `&WorldState` and return `DoorPollResult::{ None, Transition(MapDoor), Blocked { message } }`. **Cooldown** stays in `map_transition.rs`: only `Transition` sets `DOOR_TRANSITION_COOLDOWN_SECS` and clears `prev_door_overlap` to `None` (current lines 63–66). **`Blocked`:** no cooldown; after `*prev_door_overlap = now`, do **not** clear overlap — so walk-through doors do not re-fire `Blocked` every frame while standing in the rect. Transient toast: `App` holds `overworld_toast: Option<(String, remaining_secs)>` updated in `App::update`; passed into `draw_ui_pass` with layout/theme. **Content:** `mom.json` adds `set_flag:mom_intro_done` alongside `start_quest:withdraw_gold` on the `favor` node; `mumhome.firstFloor.map/doors.json` exit sets `require_state` / `deny_message`.

**Docs:** `docs/maps.md` (new door fields), `docs/ui.md` (toast pass), `docs/game.md` one-line if behaviour is player-visible.

## Steps

### gate-dialogue-api
goal: Expose world-state condition checking for non-dialogue callers (doors). No behaviour change yet.
depends_on: []
agent: implementation-agent

files_to_read:
  - src/dialogue/mod.rs
  - src/dialogue/tree.rs

context: |
  Add `pub fn world_state_satisfies(cond: Option<&str>, world_state: &WorldState) -> bool` in `dialogue/mod.rs`: `None`/empty string => true; else `tree::condition_matches(trimmed, world_state)`. Make `condition_matches` `pub(in crate::dialogue)` or `pub(super)` as needed. Refactor `state_satisfied` to call `world_state_satisfies(conv.require_state.as_deref(), world_state)`. Add a small unit test mirroring existing `state_satisfied` empty/trim behaviour if practical.

### gate-config-map-transition
goal: `MapDoor` optional `require_state` + `deny_message`; door poll returns Transition | Blocked | None with WorldState; blocked path skips transition cooldown.
depends_on: [gate-dialogue-api]
agent: implementation-agent

files_to_read:
  - src/config.rs
  - src/state/map_transition.rs
  - src/constants.rs

context: |
  `MapDoor`: `require_state: Option<String>`, `deny_message: Option<String>` with serde default None. When a poll would yield a transition, if `require_state` is Some and `!dialogue::world_state_satisfies(require_state.as_deref(), world_state)`, return `Blocked` with message from `deny_message.clone().unwrap_or_default()`; if message empty, `log::warn` and omit toast. Update `poll_map_door_transition`: add `world_state: &WorldState`, return `DoorPollResult`. Cooldown + `prev_door_overlap = None` only for `Transition` (keep this inside `map_transition.rs`, not main). For `Blocked`, leave `prev_door_overlap` as `now` so walk-through does not spam each frame.

### gate-app-renderer-ui
goal: Wire poll result in `main.rs`, toast timer in `App::update`, draw toast in renderer + layout + optional theme.
depends_on: [gate-config-map-transition]
agent: implementation-agent

files_to_read:
  - src/main.rs
  - src/gpu/renderer.rs
  - src/ui/layout.rs
  - src/ui/theme.rs
  - assets/ui/theme.json

context: |
  App: `overworld_toast: Option<(String, f32)>`. On `DoorPollResult::Blocked`, set toast text and e.g. 4.0s duration. Decrement timer in update when Overworld. Pass `overworld_toast: Option<&str>` into `draw_frame`/`draw_ui_pass`. Draw after HP (if any) / before or after dialogue panel region — use a simple bottom-centered single-line or wrapped line consistent with theme. No ECS mutation in renderer. Match existing `push_ui_text` / scaling patterns.

### gate-content-docs
goal: Author first-floor exit gate + mom flag; update docs/maps.md, docs/ui.md, docs/game.md as needed.
depends_on: [gate-app-renderer-ui]
agent: implementation-agent

files_to_read:
  - assets/dialogue/mom.json
  - assets/maps/mumhome.firstFloor.map/doors.json
  - docs/maps.md
  - docs/ui.md
  - docs/game.md

context: |
  `mom.json` `favor` effects: add `set_flag:mom_intro_done` next to existing `start_quest:withdraw_gold`. Exit door to `dustfall.junction`: `"require_state": "flag:mom_intro_done"`, `"deny_message": "Your mom wants to talk to you"` (exact user-facing string). Document new JSON fields in maps.md; ui.md note for overworld toast; game.md brief mention if appropriate.
