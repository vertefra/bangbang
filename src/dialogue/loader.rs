//! Load conversations from assets/dialogue/{id}.json.

use super::Conversation;

use crate::paths;
fn assets_dialogue_dir() -> std::path::PathBuf {
    paths::asset_root().join("dialogue")
}

/// Load conversation by id from assets/dialogue/{id}.json. Returns None if file missing or invalid.
pub fn load(conversation_id: &str) -> Option<Conversation> {
    let path = assets_dialogue_dir().join(format!("{}.json", conversation_id));
    let s = std::fs::read_to_string(&path).ok()?;
    Conversation::from_json(&s).ok()
}

