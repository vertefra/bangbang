# Plan: Dustfall Heist Scene

## Preamble

Implement the Bank Entrance heist sequence from storyboard rows 4–5. The core addition
is a lightweight data-driven **Scene system**: a new `AppState::Scene` variant that drives
a scripted sequence of NPC dialogue lines and one-shot effects (GiveSkill, SetFlag) read
from `assets/scenes/{id}.scene.json`. Scenes are triggered by proximity zones declared in
`assets/maps/{map}.map/scenes.json`, checked during the Overworld update loop alongside the
existing NPC proximity check. Scene dialogue reuses the existing typewriter + portrait
rendering infrastructure without duplicating it. A new `Rusty Peacemaker` permanent weapon
skill is added as the scene payoff. Silas and Bank Owner are authored as NPC-style characters
(config + art) used only within the scene dialogue steps (they are **not** spawned as world
entities). The default backpack seeding is removed so the player starts empty.

---

## Steps

### S1
goal: Generate Silas (The Thief) character art — walk sheet + portrait.
depends_on: []
agent: mcp-asset-creator

files_to_read:
  - assets/ASSET_STYLE_GUIDE.md

context: |
  Create pixel art for Silas, a scrappy thief character in a dusty Western setting.
  Target paths:
    - assets/npc/silas.npc/sheet.png   (walk sheet; same folder as config.json)
    - assets/npc/silas.npc/portrait.png (dialogue portrait, same size convention)
  Follow all conventions in ASSET_STYLE_GUIDE.md exactly (palette, size, format).
  Update ASSET_STYLE_GUIDE.md if any new convention is introduced.
  Do NOT create NPC config files (step S5 handles those).
  Return: the exact paths written and any style decisions made.

---

### S2
goal: Generate Bank Owner character art — walk sheet + portrait.
depends_on: []
agent: mcp-asset-creator

files_to_read:
  - assets/ASSET_STYLE_GUIDE.md

context: |
  Create pixel art for the Bank Owner — a flustered, rotund saloon-era banker.
  Target paths:
    - assets/npc/bankOwner.npc/sheet.png
    - assets/npc/bankOwner.npc/portrait.png
  Follow ASSET_STYLE_GUIDE.md conventions exactly.
  Update ASSET_STYLE_GUIDE.md if any new convention is introduced.
  Do NOT create NPC config files (step S5 handles those).
  Return: the exact paths written and any style decisions made.

---

### S3
goal: Generate Rusty Peacemaker skill icon (96×96 RGBA PNG).
depends_on: []
agent: mcp-asset-creator

files_to_read:
  - assets/ASSET_STYLE_GUIDE.md

context: |
  Create a 96×96 pixel art skill icon for "Rusty Peacemaker" — an old, worn revolver.
  Target path: assets/skills/rustyPeacemaker.skill/skill_image.png
  Follow ASSET_STYLE_GUIDE.md skill icon conventions exactly (same palette/size as
  sidearm.skill/skill_image.png and beer.skill/skill_image.png for reference).
  Update ASSET_STYLE_GUIDE.md if a new convention is introduced.
  Do NOT create the skill JSON (step S4 handles that).
  Return: the exact path written and any style decisions made.

---

### S4
goal: Implement the Scene data model and loader (new src/scene/ module).
depends_on: []
agent: implementation-agent

files_to_read:
  - docs/antipatterns.md
  - src/dialogue/tree.rs
  - src/dialogue/loader.rs
  - src/dialogue/mod.rs
  - src/paths.rs
  - src/lib.rs

context: |
  Create a new module `src/scene/` with:

  **src/scene/defs.rs**
  ```rust
  pub enum SceneStep {
      Dialogue {
          speaker: String,
          portrait: Option<String>,  // asset key for portrait; None = no portrait
          lines: Vec<String>,
      },
      GiveSkill { skill_id: String },
      SetFlag { flag: String },
  }

  pub struct SceneDef {
      pub id: String,
      pub steps: Vec<SceneStep>,
  }
  ```
  JSON format for assets/scenes/{id}.scene.json:
  ```json
  {
    "id": "theHeist",
    "steps": [
      { "type": "dialogue", "speaker": "Silas", "portrait": "silas", "lines": ["Watch it, kid!"] },
      { "type": "give_skill", "skill_id": "rustyPeacemaker" },
      { "type": "set_flag", "flag": "heist_played" }
    ]
  }
  ```
  Use serde with `#[serde(tag = "type", rename_all = "snake_case")]` on SceneStep.

  **src/scene/loader.rs**
  Load `assets/scenes/{id}.scene.json` → `Result<SceneDef, SceneLoadError>`.
  All errors are explicit (antipattern #1). No silent fallback.
  Use `crate::paths::asset_root()` for all path construction (antipattern #6).

  **src/scene/mod.rs**
  ```rust
  pub struct SceneCache {
      cache: HashMap<String, SceneDef>,
  }
  impl SceneCache {
      pub fn new() -> Self { ... }
      pub fn get_or_load(&mut self, id: &str) -> Result<&SceneDef, SceneLoadError>
  }
  ```
  Load-on-demand (same pattern as ConversationCache in dialogue/mod.rs).

  Register the module in src/lib.rs.

  Constraints:
  - No ECS interaction (pure data model + I/O).
  - No game logic.
  - Return types must implement std::error::Error or be String-based (consistent with rest of codebase).
  Return: files created/modified, public types and their signatures.

---

### S5
goal: Add NPC config files for Silas and Bank Owner; add Rusty Peacemaker skill JSON.
depends_on: [S1, S2, S3]
agent: implementation-agent

files_to_read:
  - docs/npc.md
  - docs/skills.md
  - assets/npc/docSawbones.npc/config.json
  - assets/skills/sidearm.skill/config.json
  - assets/skills/beer.skill/config.json

context: |
  Three data files to create:

  **assets/npc/silas.npc/config.json**
  Scale and color for Silas. conversation_id is NOT needed (Silas only appears in scene
  steps, not as a world NPC). Use a warm-toned color that reads as a thief/rogue character.
  Reference docSawbones.npc/config.json for format.

  **assets/npc/bankOwner.npc/config.json**
  Same format. The Bank Owner only appears in scene dialogue steps, not as a world NPC.

  **assets/skills/rustyPeacemaker.skill/config.json**
  A permanent weapon skill (subcategory: "weapon"). Effects: deal_damage to opponent,
  amount slightly higher than sidearm (sidearm = reference — check assets/skills/sidearm.skill/config.json
  for its amount). Rusty Peacemaker is the first story-earned gun; it can be equal to or
  slightly stronger than sidearm. No charges_default (permanent). Id must be "rustyPeacemaker"
  (matches the skill_image path from S3).

  Constraints:
  - Do NOT spawn Silas or Bank Owner as world NPCs anywhere. These configs are only used for
    portrait lookup in scene dialogue rendering.
  - rustyPeacemaker.json must pass SkillDef::load() validation (non-empty effects, amount >= 0).
  Return: each file path and its content.

---

### S6
goal: Add AppState::Scene variant, scene trigger config loading, SceneCache in App, and all wiring so the codebase compiles end-to-end after this step.
depends_on: [S4]
agent: implementation-agent

files_to_read:
  - docs/antipatterns.md
  - docs/maps.md
  - src/state/app.rs
  - src/state/overworld.rs
  - src/state/world.rs
  - src/config.rs
  - src/map_loader.rs
  - src/main.rs
  - src/constants.rs

context: |
  This step must leave the codebase in a **compiling state**. All wiring changes go here.

  ## 1. MapSceneTrigger (src/config.rs)
  Add:
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct MapSceneTrigger {
      pub scene_id: String,
      pub trigger_position: [f32; 2],
      pub trigger_radius: f32,
      pub require_not_flag: Option<String>,
  }
  ```

  ## 2. MapData + MapContext scene_triggers (src/map_loader.rs, src/config.rs, src/main.rs)
  Add `pub scene_triggers: Vec<MapSceneTrigger>` to BOTH:
  - `MapData` (the loader result struct)
  - `MapContext` (the App-level struct holding `tilemap` in main.rs)
  Load from `assets/maps/{id}.map/scenes.json`. Missing file → empty Vec (same pattern as
  missing npc.json). Invalid JSON → MapLoadError (fail loudly, antipattern #1).
  In `apply_map_transition()` AND `apply_load_game()` in main.rs, add:
  `self.map.scene_triggers = map_data.scene_triggers.clone();`
  (alongside the existing field assignments like tilemap, doors, props).
  This ensures the correct triggers are loaded for each map and don't persist stale across transitions.

  ## 3. AppState::Scene (src/state/app.rs)
  Add variant:
  ```rust
  Scene {
      scene_id: String,
      step: usize,
      line_index: u32,
      stream_visible: u32,
      stream_acc: f32,
  },
  ```
  Update ALL FIVE `AppState::Overworld { .. }` construction sites — they all name fields
  explicitly and will fail to compile if the new field is missing. Add:
  ```rust
  scene_trigger_cooldown: f32,
  ```
  to the `Overworld` variant. Sites and their initial values:
  - `AppState::default()` in `app.rs` → `scene_trigger_cooldown: SCENE_TRIGGER_COOLDOWN_SECS`
  - Post-dialogue exit in `update_dialogue()` in `app.rs` → `scene_trigger_cooldown: 0.0`
  - Post-scene exit in `update_scene()` in `app.rs` → `scene_trigger_cooldown: SCENE_TRIGGER_COOLDOWN_SECS`
    (intentional: scene may have no require_not_flag; cooldown prevents instant re-fire; the
    SetFlag step in scene data is the replay gate, so re-triggering after cooldown is safe)
  - `apply_map_transition()` in `main.rs` → `scene_trigger_cooldown: SCENE_TRIGGER_COOLDOWN_SECS`
  - `apply_load_game()` in `main.rs` → `scene_trigger_cooldown: SCENE_TRIGGER_COOLDOWN_SECS`
  Add `SCENE_TRIGGER_COOLDOWN_SECS: f32 = 1.0` to src/constants.rs.

  ## 4. update_scene() (src/state/app.rs)
  Thread `scene_cache: &mut SceneCache` into `AppState::update()` signature (alongside the
  existing `dialogue_cache`). Add a `Scene { .. }` arm to the dispatch that calls `update_scene()`.
  `update_scene()` logic:
  - Look up scene in scene_cache; if not found: log error, transition to Overworld (no crash).
  - If current step is Dialogue: advance typewriter (stream_acc + dt → stream_visible).
    Space/Enter: if still typing, reveal full line; if full line shown, advance line_index;
    if last line in this Dialogue step, advance `step` and reset line_index/stream to 0.
  - If current step is GiveSkill: call `skills::give_skill(world, skill_registry, skill_id)?`
    (stub the signature now; implementation body goes in S7). Advance step.
  - If current step is SetFlag: call `world_state.set_flag(flag)`. Advance step.
  - Non-Dialogue steps execute in a loop (no frame stall).
  - If step >= steps.len(): transition to `AppState::Overworld { last_near_npc: true, backpack_open: false, scene_trigger_cooldown: 0.0 }`.

  ## 5. Scene proximity check (src/state/app.rs — update_overworld)
  DO NOT change `overworld::update()` return type. Instead, perform the scene trigger
  check INSIDE `update_overworld()` after the existing overworld::update() call, using
  the loaded scene trigger slice passed as a new parameter:
  - Add `scene_triggers: &[MapSceneTrigger]` and `scene_cache: &mut SceneCache` as parameters
    to `update_overworld()`.
  - Decrement `scene_trigger_cooldown` each frame (clamp to 0.0).
  - While cooldown > 0: skip scene trigger check.
  - Otherwise: iterate triggers; compute distance from player Transform.position to trigger_position;
    if within trigger_radius AND (require_not_flag is None OR !world_state.has_flag(flag)):
    load scene from scene_cache; if Ok, transition to AppState::Scene { scene_id, step: 0, ... }; return.
  - Update the call site in AppState::update() to pass scene_triggers and scene_cache.

  ## 6. SceneCache in App resources (src/main.rs)
  Add `scene_cache: SceneCache` to the `Resources` struct (wherever dialogue_cache lives).
  Initialize with `SceneCache::new()` in main().
  Thread into `AppState::update()` call alongside `dialogue_cache`.

  Constraints:
  - Antipattern #4: no bool fields added to App struct. scene_trigger_cooldown on Overworld variant.
  - Antipattern #3: no logic in draw_frame.
  - give_skill() can be a stub (returns Ok(())) this step; S7 fills the body.
  - Codebase MUST compile after this step (all match arms updated, all call sites updated).
  Return: all files modified, new types, function signatures, and confirmation it compiles (cargo check).

---

### S7
goal: Wire Scene rendering into App::draw (inline in RedrawRequested handler) and implement the give_skill helper body.
depends_on: [S4, S6]
agent: implementation-agent

files_to_read:
  - src/main.rs
  - src/gpu/frame_context.rs
  - src/skills/backpack_runtime.rs
  - src/ecs/components.rs
  - src/assets.rs

context: |
  ## 1. Scene draw wiring (src/main.rs, RedrawRequested handler)
  The existing `dialogue_display_text()` method on AppState cannot be extended to Scene without
  breaking its signature. Instead, in the `RedrawRequested` / draw path in main.rs, add an
  INLINE arm for AppState::Scene alongside the existing Dialogue arm:
  - When current scene step is SceneStep::Dialogue { speaker, portrait, lines, .. }:
    - `dialogue_npc_id`: set to speaker string (used for panel label)
    - `dialogue_display_text`: first `stream_visible` chars of `lines[line_index]`
    - `dialogue_message`: full `lines[line_index]`
    - `dialogue_portrait_texture`: portrait lookup using `portrait` as the character id
      (call the same portrait resolution functions used by the Dialogue state — see src/assets.rs)
  - When current scene step is NOT Dialogue (or step is out of range): do not populate dialogue
    fields (same as Overworld draw path — no panel).
  Compute display text inline — do NOT call `self.game.app_state.dialogue_display_text(...)`.

  ## 2. give_skill helper body (src/skills/backpack_runtime.rs)
  S6 created a stub. Fill it:
  ```rust
  pub fn give_skill(world: &mut World, registry: &SkillRegistry, skill_id: &str) -> Result<(), String> {
      // Validate skill exists
      let def = registry.get(skill_id).ok_or_else(|| format!("give_skill: unknown skill '{}'", skill_id))?;
      // Find player
      let player = player_entity(world).ok_or_else(|| "give_skill: no Player entity".to_string())?;
      let mut backpack = world.get::<&mut Backpack>(player)
          .map_err(|_| "give_skill: player has no Backpack".to_string())?;
      // Add permanent skill (idempotent)
      if !backpack.permanent.contains(&skill_id.to_string()) {
          backpack.permanent.push(skill_id.to_string());
      }
      // Auto-equip if weapon and nothing equipped
      if def.subcategory.as_deref() == Some("weapon") && backpack.equipped_weapon_id.is_none() {
          backpack.equipped_weapon_id = Some(skill_id.to_string());
      }
      Ok(())
  }
  ```
  Fail explicitly on unknown skill or missing player/backpack (antipattern #1).

  Constraints:
  - Do not duplicate dialogue rendering logic — reuse exact same FrameContext fields.
  - No game logic in draw_frame.
  - give_skill must not silently succeed for unknown skills.
  Return: files modified, functions changed.

---

### S8
goal: Author scene data files (theHeist.scene.json + junction scenes.json trigger).
depends_on: [S4]
agent: implementation-agent

files_to_read:
  - assets/maps/dustfall.junction.map/props.json
  - plan.heist-scene.md

context: |
  ## 1. assets/scenes/theHeist.scene.json
  Using the SceneDef format from S4 (type-tagged steps):
  - Step 1: Dialogue, speaker "Silas", portrait "silas", lines from storyboard row 4:
    "Watch it, kid! I'm on a schedule."
    "Don't look at me like that—this gold is insured! Insurance is the real thief here!"
    "(Silas flees. A metallic clink echoes as he drops something.)"
  - Step 2: Dialogue, speaker "Bank Owner", portrait "bankOwner", lines from storyboard row 5:
    "Coward! Someone stop him!"
    "Sighs. This town needs a Sheriff with some grit. Our current one is half-blind."
    "Is that his iron? Pick it up, kid. If you're going to the Old Mine to get that gold back, you'll need more than luck."
  - Step 3: GiveSkill, skill_id "rustyPeacemaker"
  - Step 4: SetFlag, flag "heist_played"

  ## 2. assets/maps/dustfall.junction.map/scenes.json
  One trigger:
  - scene_id: "theHeist"
  - trigger_position: center of bank prop area (bank prop is at [1104, 464] per props.json;
    use a position just south of that, e.g. [1104, 560], so player triggers it walking toward the bank)
  - trigger_radius: 96 (one and a half tiles at typical 64px tile_size)
  - require_not_flag: "heist_played"

  Create the assets/scenes/ directory if it doesn't exist.
  Return: exact file paths and complete JSON content.

---

### S9
goal: Remove default backpack seeding — player starts with empty backpack.
depends_on: []
agent: implementation-agent

files_to_read:
  - assets/game.json
  - src/skills/backpack_runtime.rs
  - src/main.rs

context: |
  Set "seed_demo_backpack": false in assets/game.json.
  The seed_demo_backpack flag gates a call in main.rs to skills::seed_demo_backpack.
  With flag false, the player spawns with the empty Backpack that setup_world initializes.

  **Required checks**: Also read src/skills/backpack_view.rs and src/gpu/pass_backpack.rs.
  Verify that an empty Backpack (no permanent, no usable, equipped_weapon_id: None) does NOT
  panic at startup or when B is pressed:
  - normalize_equipped_weapon() on an empty permanent list
  - apply_backpack_hotkey() / cycle_equipped_weapon() with no weapons
  - pass_backpack draw loop with zero-length slot lists
  If any of these would panic, add explicit early-return guards and report the specific file+line fixed.

  Health cold-start (5/5) is unchanged.
  Do NOT delete the seed_demo_backpack function — useful for testing.
  Just set the flag to false.
  Return: files modified and any panic-risk invariants found (with fixes applied).

---

### S10
goal: Update docs (architecture.md, game.md, skills.md, maps.md) to reflect all changes.
depends_on: [S4, S5, S6, S7, S8, S9]
agent: implementation-agent

files_to_read:
  - docs/architecture.md
  - docs/game.md
  - docs/skills.md
  - docs/maps.md
  - plan.heist-scene.md

context: |
  Update documentation to reflect the feature additions. Do not rewrite docs wholesale —
  add/update only what changed.

  docs/architecture.md:
  - Add src/scene/ to the Crate Layout table (mod.rs, defs.rs, loader.rs).
  - Note that AppState now has a Scene variant alongside Overworld, Dialogue, Duel.

  docs/game.md:
  - Update "Current state" to note the Scene system and the heist scene.
  - Update the Plan table: mark Phase 4 (Story & dialogue) as having scene support.
  - Update Todo section.

  docs/skills.md:
  - Add rustyPeacemaker to built-in skills list.
  - Document the "collect skill" mechanic: GiveSkill scene step awards skills automatically
    at scene end; this is the chosen implementation (explicit document: "automatic at scene
    completion via GiveSkill step").
  - Note give_skill helper function.

  docs/maps.md:
  - Add scenes.json section (parallel to npc.json / props.json / doors.json). Document:
    - file format: array of MapSceneTrigger
    - fields: scene_id, trigger_position, trigger_radius, require_not_flag
    - missing file: no triggers (not an error)
    - invalid JSON: MapLoadError

  Return: which docs were updated and the specific sections changed.
