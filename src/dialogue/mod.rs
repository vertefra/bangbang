//! Dialogue: load conversations, resolve the current line, advance with effects.
//!
//! ## Assets
//!
//! Files: `assets/dialogue/{conversation_id}.json`. The `conversation_id` string comes from the merged
//! NPC config ([`crate::config::NpcConfig`]) when the world is built; see repository `docs/npc.md`.
//!
//! ## Runtime
//!
//! [`ConversationCache`] avoids re-reading disk for loaded and generated conversations. The private
//! `loader` submodule maps JSON to [`tree::Conversation`].
//! Story state (flags, path) drives branch conditions and receives effects on advance.

mod loader;
mod tree;

pub use tree::{Branch, Conversation, Node};

use crate::state::WorldState;
use std::collections::HashMap;

/// Cache for loaded conversations to avoid redundant IO.
#[derive(Debug, Default)]
pub struct ConversationCache {
    cache: HashMap<String, Conversation>,
}

impl ConversationCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_load(&mut self, id: &str) -> Option<&Conversation> {
        if !self.cache.contains_key(id) {
            if let Some(conv) = loader::load(id) {
                self.cache.insert(id.to_string(), conv);
            }
        }
        self.cache.get(id)
    }

    pub fn insert_generated(&mut self, id: impl Into<String>, conv: Conversation) -> &Conversation {
        let id = id.into();
        self.cache.entry(id).or_insert(conv)
    }
}

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

/// Apply effect string to story state. Format: "set_flag:name", "set_path:name", "start_quest:id", "complete_quest:id".
fn apply_effect(effect: &str, world_state: &mut WorldState) {
    let effect = effect.trim();
    if let Some(flag) = effect.strip_prefix("set_flag:") {
        world_state.set_flag(flag.trim());
        return;
    }
    if let Some(path) = effect.strip_prefix("set_path:") {
        world_state.choose_path(path.trim());
        return;
    }
    if let Some(id) = effect.strip_prefix("start_quest:") {
        world_state.start_quest(id.trim());
        return;
    }
    if let Some(id) = effect.strip_prefix("complete_quest:") {
        world_state.complete_quest(id.trim());
        return;
    }
    log::warn!("dialogue effect has unknown prefix: {:?}", effect);
}

/// Returns true if the conversation's `require_state` condition is satisfied (or if there is no condition).
pub fn state_satisfied(conv: &Conversation, world_state: &WorldState) -> bool {
    let req = match &conv.require_state {
        Some(r) => r,
        None => return true,
    };
    let req = req.trim();
    if req.is_empty() {
        return true;
    }
    tree::condition_matches(req, world_state)
}

/// First node to show when opening a conversation: follows `start`, then auto-advances through any
/// nodes with zero lines (typical **router** node that only has `branches`). Returns `None` if the
/// tree ends before any line is shown (malformed data).
pub fn entry_point(conv: &Conversation, world_state: &mut WorldState) -> Option<(String, u32)> {
    if !state_satisfied(conv, world_state) {
        return None;
    }

    const MAX_EMPTY_HOPS: u32 = 64;
    let mut node_id = conv.start.clone();
    let mut line_index = 0u32;
    let mut hops = 0u32;
    loop {
        if conv.line_count(&node_id) > 0 {
            return Some((node_id, line_index));
        }
        hops += 1;
        if hops > MAX_EMPTY_HOPS {
            log::warn!(
                "dialogue entry_point: exceeded {} empty nodes from start {:?}",
                MAX_EMPTY_HOPS,
                conv.start
            );
            return None;
        }
        let result = advance(conv, &node_id, line_index, world_state);
        if result.finished {
            return None;
        }
        node_id = result.node_id;
        line_index = result.line_index;
    }
}

/// Advance dialogue: if more lines in current node, advance line; else apply effects, go to next node.
/// Returns next node_id and line_index; finished = true when there is no next node.
pub fn advance(
    conv: &Conversation,
    node_id: &str,
    line_index: u32,
    world_state: &mut WorldState,
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
        apply_effect(effect, world_state);
    }
    let next_id = node.resolve_next(world_state);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_conv_with_req(req: Option<&str>) -> Conversation {
        let mut conv = Conversation::one_line("dummy");
        conv.require_state = req.map(|s| s.to_string());
        conv
    }

    #[test]
    fn test_quest_effects() {
        let mut w = WorldState::default();
        apply_effect("start_quest:find_dog", &mut w);
        assert!(w.quest_active("find_dog"));
        assert!(!w.quest_complete("find_dog"));

        apply_effect("complete_quest:find_dog", &mut w);
        assert!(!w.quest_active("find_dog"));
        assert!(w.quest_complete("find_dog"));
    }

    #[test]
    fn test_require_state_satisfied() {
        let w = WorldState::default();
        let conv_empty = dummy_conv_with_req(None);
        let conv_flag = dummy_conv_with_req(Some("flag:known"));
        let conv_quest = dummy_conv_with_req(Some("quest_active:run"));

        assert!(state_satisfied(&conv_empty, &w));
        assert!(!state_satisfied(&conv_flag, &w));
        assert!(!state_satisfied(&conv_quest, &w));

        let mut w2 = WorldState::default();
        w2.set_flag("known");
        w2.start_quest("run");

        assert!(state_satisfied(&conv_flag, &w2));
        assert!(state_satisfied(&conv_quest, &w2));
    }

    #[test]
    fn test_entry_point_with_require_state() {
        let conv = dummy_conv_with_req(Some("flag:met"));
        let mut w = WorldState::default();

        // fails requirement
        assert!(entry_point(&conv, &mut w).is_none());

        // passes requirement
        w.set_flag("met");
        assert!(entry_point(&conv, &mut w).is_some());
    }

    #[test]
    fn generated_conversations_use_separate_cache_keys() {
        let mut cache = ConversationCache::new();
        assert!(cache.get_or_load("__missing_real_conversation__").is_none());
        cache.insert_generated(
            "__missing_real_conversation___fallback",
            Conversation::one_line("placeholder"),
        );
        assert!(cache.get_or_load("__missing_real_conversation__").is_none());
        assert_eq!(
            current_display(
                cache
                    .get_or_load("__missing_real_conversation___fallback")
                    .expect("generated conversation should be cached"),
                "start",
                0
            ),
            Some("placeholder")
        );
    }

    #[test]
    fn test_quest_conditions_in_dialogue() {
        let json = r#"{
            "start": "check",
            "nodes": {
                "check": {
                    "branches": [
                        { "condition": "quest_active:test", "next": "is_active" },
                        { "condition": "quest_complete:test", "next": "is_complete" },
                        { "next": "default" }
                    ]
                },
                "is_active": { "line": "active", "next": null },
                "is_complete": { "line": "complete", "next": null },
                "default": { "line": "none", "next": null }
            }
        }"#;
        let conv = tree::Conversation::from_json(json).unwrap();

        // Default
        let mut w = WorldState::default();
        let (node, _) = entry_point(&conv, &mut w).unwrap();
        assert_eq!(node, "default");

        // Active
        w.start_quest("test");
        let (node, _) = entry_point(&conv, &mut w).unwrap();
        assert_eq!(node, "is_active");

        // Complete
        w.complete_quest("test");
        let (node, _) = entry_point(&conv, &mut w).unwrap();
        assert_eq!(node, "is_complete");
    }
}
