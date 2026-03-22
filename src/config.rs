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

use serde::Deserialize;

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
