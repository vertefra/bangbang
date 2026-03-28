//! # Config
//!
//! Shared **serde** types for game content JSON.
//!
//! ## NPCs
//!
//! - **[`MapNpcEntry`]** — one row in `assets/maps/{map}.map/npc.json` (`id` + world `position`).
//! - **[`CharacterNpcConfig`]** — `assets/npc/{id}.npc/config.json` (or legacy `{id}.npc.json`): visual + dialogue id.
//! - **[`NpcConfig`]** — merged at load time (`map_loader`): position from the map, rest from the character file.
//!
//! Authoring reference: repository `docs/npc.md`. Map doors: `docs/maps.md` (`MapDoor`).
//!
//! ## Game bootstrap
//!
//! - **[`GameConfig`]** — `assets/game.json`: initial map id, optional demo backpack seed, window title.

use serde::Deserialize;
use std::path::PathBuf;

/// One NPC instance on a map: `assets/maps/{map}.map/npc.json` array element.
///
/// `id` selects `assets/npc/{id}.npc/config.json` (or legacy `assets/npc/{id}.npc.json`).
/// `position` is the only placement source for that NPC; it is **not** read from the character config file.
#[derive(Debug, Clone, Deserialize)]
pub struct MapNpcEntry {
    pub id: String,
    pub position: [f32; 2],
}

/// One static prop on a map: `assets/maps/{map}.map/props.json` array element.
///
/// `id` selects `assets/props/{id}.prop/sheet.png` by convention (+ optional `sheet.json` grid).
/// Legacy `assets/props/{id}/` folders are still accepted during migration. No dialogue or AI.
#[derive(Debug, Clone, Deserialize)]
pub struct MapPropEntry {
    pub id: String,
    pub position: [f32; 2],
    #[serde(default = "default_prop_scale")]
    pub scale: [f32; 2],
}

fn default_prop_scale() -> [f32; 2] {
    [1.0, 1.0]
}

/// One door / map transition in `assets/maps/{id}.map/doors.json`. `rect` is `[min_x, min_y, width, height]` in world units (player position must lie inside).
#[derive(Debug, Clone, Deserialize)]
pub struct MapDoor {
    pub rect: [f32; 4],
    pub to_map: String,
    pub spawn: [f32; 2],
    #[serde(default = "default_door_require_confirm")]
    pub require_confirm: bool,
    /// When set, transition only if this matches [`crate::dialogue::world_state_satisfies`]
    /// (e.g. `flag:mom_intro_done`). Otherwise the door shows [`Self::deny_message`] and does not transition.
    #[serde(default)]
    pub require_state: Option<String>,
    /// Shown as a transient overworld banner when `require_state` is set and not satisfied.
    #[serde(default)]
    pub deny_message: Option<String>,
    /// Optional door prop id. `"south"` resolves to `assets/props/south.door/` and
    /// `"southHeavy"` resolves to `assets/props/southHeavy.door/` by convention.
    /// `"none"` (or a missing field) means transition-only with no door sprite.
    /// `alias = "visual"` keeps older maps readable during migration.
    #[serde(default, alias = "visual")]
    pub prop: Option<String>,
}

fn default_door_require_confirm() -> bool {
    true
}

impl MapDoor {
    /// User-facing door prop id from `doors.json`, normalized so `"none"` / empty behaves like no prop.
    pub fn prop_id(&self) -> Option<&str> {
        match self.prop.as_deref().map(str::trim) {
            Some("") | Some("none") | None => None,
            Some(id) => Some(id),
        }
    }
}

/// Character definition: `assets/npc/{id}.npc/config.json` preferred, else `assets/npc/{id}.npc.json`.
///
/// Unknown JSON keys are ignored by `serde`. There is **no** `position` field here — use the map’s `npc.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct CharacterNpcConfig {
    /// World-space scale factors on the NPC [`crate::ecs::Transform`].
    #[serde(default = "default_scale")]
    pub scale: [f32; 2],
    /// RGBA tint / solid color when no character sheet loads ([`crate::ecs::Sprite`]).
    #[serde(default = "default_color")]
    pub color: [f32; 4],
    /// Base name for `assets/dialogue/{conversation_id}.json`. If `None`, the map entry’s `id` is used.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// Merged NPC data after `map_loader` combines [`MapNpcEntry`] + [`CharacterNpcConfig`].
///
/// Consumed by [`crate::ecs::world::setup_world`] to spawn entities.
#[derive(Debug, Clone)]
pub struct NpcConfig {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [f32; 4],
    /// Loaded conversation file stem; may differ from the character folder `id` when set in [`CharacterNpcConfig`].
    pub conversation_id: String,
}

fn default_scale() -> [f32; 2] {
    [0.5, 0.5]
}

fn default_color() -> [f32; 4] {
    [0.2, 0.6, 1.0, 1.0]
}

/// One scene proximity trigger in `assets/maps/{id}.map/scenes.json`.
///
/// When the player is within `trigger_radius` world units of `trigger_position`, the named
/// scene starts. `require_not_flag` gates repeat plays: if the flag is set the trigger is skipped.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MapSceneTrigger {
    pub scene_id: String,
    pub trigger_position: [f32; 2],
    pub trigger_radius: f32,
    #[serde(default)]
    pub require_not_flag: Option<String>,
}

/// Top-level game bootstrap: `assets/game.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct GameConfig {
    /// Map id passed to [`crate::map_loader::load_map`] (e.g. `mumhome.secondFloor`).
    pub start_map: String,
    /// When true, [`crate::skills::seed_demo_backpack`] runs at startup.
    pub seed_demo_backpack: bool,
    /// Window title: applied in the binary when creating the window (`Window::default_attributes().with_title(...)`).
    pub window_title: String,
}

#[derive(Debug)]
pub enum GameConfigError {
    Io(std::io::Error, PathBuf),
    Json(serde_json::Error, PathBuf),
    Invalid { path: PathBuf, message: String },
}

impl std::fmt::Display for GameConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e, p) => write!(f, "IO error at {}: {}", p.display(), e),
            Self::Json(e, p) => write!(f, "JSON error at {}: {}", p.display(), e),
            Self::Invalid { path, message } => {
                write!(f, "invalid game config at {}: {}", path.display(), message)
            }
        }
    }
}

impl std::error::Error for GameConfigError {}

impl GameConfig {
    /// Load and validate `assets/game.json`. Fails if the file is missing, not valid JSON, or has empty required strings.
    pub fn load() -> Result<Self, GameConfigError> {
        let path = crate::paths::asset_root().join("game.json");
        let s =
            std::fs::read_to_string(&path).map_err(|e| GameConfigError::Io(e, path.clone()))?;
        let cfg: GameConfig =
            serde_json::from_str(&s).map_err(|e| GameConfigError::Json(e, path.clone()))?;
        if cfg.start_map.trim().is_empty() {
            return Err(GameConfigError::Invalid {
                path: path.clone(),
                message: "start_map must be non-empty".into(),
            });
        }
        if cfg.window_title.trim().is_empty() {
            return Err(GameConfigError::Invalid {
                path,
                message: "window_title must be non-empty".into(),
            });
        }
        Ok(cfg)
    }
}
