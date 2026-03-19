//! Dialogue module: load conversations, resolve current line, advance with effects.
//!
//! Conversations live in assets/dialogue/{conversation_id}.json. Story state (flags, path)
//! drives branch conditions and receives effects on advance.

mod loader;
mod tree;

pub use tree::{Branch, Conversation, Node};

use crate::state::StoryState;

/// Result of advancing dialogue: next node, line index within that node, and whether conversation ended.
#[derive(Debug)]
pub struct AdvanceResult {
    pub node_id: String,
    pub line_index: u32,
    pub finished: bool,
}

/// Load conversation by id. Returns None if file missing or invalid.
pub fn load(conversation_id: &str) -> Option<Conversation> {
    loader::load(conversation_id)
}

/// Load conversation by id; if no file, use a one-line conversation from fallback_line.
pub fn load_or_fallback(conversation_id: &str, fallback_line: &str) -> Conversation {
    loader::load_or_fallback(conversation_id, fallback_line)
}

/// Current line to display for (conversation, node_id, line_index). Returns None if invalid.
pub fn current_display<'a>(
    conv: &'a Conversation,
    node_id: &str,
    line_index: u32,
) -> Option<&'a str> {
    let lines = conv.node_lines(node_id);
    let i = line_index as usize;
    if i < lines.len() {
        Some(lines[i])
    } else {
        None
    }
}

/// Apply effect string to story state. Format: "set_flag:name" or "set_path:name".
fn apply_effect(effect: &str, story: &mut StoryState) {
    let effect = effect.trim();
    if let Some(flag) = effect.strip_prefix("set_flag:") {
        story.set_flag(flag.trim());
        return;
    }
    if let Some(path) = effect.strip_prefix("set_path:") {
        story.choose_path(path.trim());
    }
}

/// Advance dialogue: if more lines in current node, advance line; else apply effects, go to next node.
/// Returns next node_id and line_index; finished = true when there is no next node.
pub fn advance(
    conv: &Conversation,
    node_id: &str,
    line_index: u32,
    story: &mut StoryState,
) -> AdvanceResult {
    let count = conv.line_count(node_id) as u32;
    if line_index + 1 < count {
        return AdvanceResult {
            node_id: node_id.to_string(),
            line_index: line_index + 1,
            finished: false,
        };
    }
    let node = match conv.get_node(node_id) {
        Some(n) => n,
        None => {
            return AdvanceResult {
                node_id: node_id.to_string(),
                line_index: 0,
                finished: true,
            };
        }
    };
    for effect in &node.effects {
        apply_effect(effect, story);
    }
    let next_id = node.resolve_next(story);
    match next_id {
        Some(id) => AdvanceResult {
            node_id: id,
            line_index: 0,
            finished: false,
        },
        None => AdvanceResult {
            node_id: node_id.to_string(),
            line_index: 0,
            finished: true,
        },
    }
}
