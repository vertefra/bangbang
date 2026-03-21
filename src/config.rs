//! # Config
//!
//! Shared config types for game content. Map npc.json defines placement (id + position);
//! character files define scale, color, dialogue.

use serde::Deserialize;

/// One NPC placement in a map's npc.json. Position is in world units.
#[derive(Debug, Clone, Deserialize)]
pub struct MapNpcEntry {
    pub id: String,
    pub position: [f32; 2],
}

/// Character definition in assets/npc/{id}.npc.json. No position (defined by map).
#[derive(Debug, Clone, Deserialize)]
pub struct CharacterNpcConfig {
    #[serde(default = "default_scale")]
    pub scale: [f32; 2],
    #[serde(default = "default_color")]
    pub color: [f32; 4],
    /// Conversation id for assets/dialogue/{id}.json. If absent, NPC id is used.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// Merged NPC config: position from map, rest from character. Used at runtime.
#[derive(Debug, Clone)]
pub struct NpcConfig {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [f32; 4],
    /// Conversation id (from config or NPC id).
    pub conversation_id: String,
}

fn default_scale() -> [f32; 2] {
    [0.5, 0.5]
}

fn default_color() -> [f32; 4] {
    [0.2, 0.6, 1.0, 1.0]
}
