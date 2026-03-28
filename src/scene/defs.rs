//! Scene data types: a linear sequence of steps deserialized from JSON.

use serde::Deserialize;

/// One step in a scripted scene.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneStep {
    Dialogue {
        speaker: String,
        /// Character id used for portrait lookup. `None` = no portrait shown.
        #[serde(default)]
        portrait: Option<String>,
        lines: Vec<String>,
    },
    GiveSkill {
        skill_id: String,
    },
    SetFlag {
        flag: String,
    },
}

/// A complete scene: an ordered list of steps.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneDef {
    pub id: String,
    pub steps: Vec<SceneStep>,
}
