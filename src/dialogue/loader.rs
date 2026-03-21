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

/// Load conversation by id; if no file exists, return a one-line conversation from fallback_line.
pub fn load_or_fallback(conversation_id: &str, fallback_line: &str) -> Conversation {
    load(conversation_id).unwrap_or_else(|| Conversation::one_line(fallback_line))
}
