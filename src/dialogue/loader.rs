//! Load conversations from assets/dialogue/{id}.json.

use super::Conversation;

use crate::paths;
fn assets_dialogue_dir() -> std::path::PathBuf {
    paths::asset_root().join("dialogue")
}

/// Load conversation by id from assets/dialogue/{id}.json.
/// Returns `None` if the file is missing. Invalid JSON or unknown condition/effect strings are logged and yield `None`.
pub fn load(conversation_id: &str) -> Option<Conversation> {
    let path = assets_dialogue_dir().join(format!("{}.json", conversation_id));
    let s = std::fs::read_to_string(&path).ok()?;
    match Conversation::from_json(&s) {
        Ok(c) => Some(c),
        Err(e) => {
            log::error!(
                "dialogue parse failed for {} ({}): {}",
                conversation_id,
                path.display(),
                e
            );
            None
        }
    }
}
