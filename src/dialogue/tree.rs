//! Conversation tree types: nodes, branches, effects. Deserialized from JSON.
//!
//! ## Conditions (branch)
//!
//! | Prefix | Example | Checks |
//! |--------|---------|--------|
//! | `flag:` | `flag:met_sheriff` | `WorldState::has_flag` |
//! | `path:` | `path:bandit` | `WorldState::path()` |
//! | `quest_active:` | `quest_active:withdraw_gold` | `WorldState::quest_active` |
//! | `quest_complete:` | `quest_complete:withdraw_gold` | `WorldState::quest_complete` |
//!
//! ## Effects (node)
//!
//! | Prefix | Example | Calls |
//! |--------|---------|-------|
//! | `set_flag:` | `set_flag:met_sheriff` | `WorldState::set_flag` |
//! | `set_path:` | `set_path:bandit` | `WorldState::choose_path` |
//! | `start_quest:` | `start_quest:withdraw_gold` | `WorldState::start_quest` |
//! | `complete_quest:` | `complete_quest:withdraw_gold` | `WorldState::complete_quest` |

use serde::Deserialize;
use std::collections::HashMap;

/// One conversation: start node id and all nodes by id.
///
/// Optional `require_state` gates entry: if present, [`crate::dialogue::state_satisfied`] must
/// return `true` before the tree opens. When gating fails, `default_line` is shown instead.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub start: String,
    pub nodes: HashMap<String, Node>,
    /// Optional condition that must hold for this conversation to open (e.g. `"quest_active:withdraw_gold"`).
    /// Uses the same syntax as branch conditions. `None` = always open.
    pub require_state: Option<String>,
    /// Line shown when `require_state` is not satisfied. If `None`, dialogue is silently skipped.
    pub default_line: Option<String>,
}

/// One node: single line, or multiple lines; then next node or branches.
#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    #[serde(default)]
    pub line: Option<String>,
    #[serde(default)]
    pub lines: Vec<String>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub branches: Vec<Branch>,
    #[serde(default)]
    pub effects: Vec<String>,
}

/// Conditional branch: optional condition, next node id.
///
/// Supported condition prefixes: `flag:`, `path:`, `quest_active:`, `quest_complete:`.
/// A branch with no condition always matches (use as default/fallback after specific conditions).
#[derive(Debug, Clone, Deserialize)]
pub struct Branch {
    #[serde(default)]
    pub condition: Option<String>,
    pub next: String,
}

#[derive(Debug, Deserialize)]
struct ConversationJson {
    pub start: String,
    pub nodes: HashMap<String, Node>,
    #[serde(default)]
    pub require_state: Option<String>,
    #[serde(default)]
    pub default_line: Option<String>,
}

impl Conversation {
    /// Lines for this node: either single "line" or "lines" array. Empty if neither set.
    pub fn node_lines(&self, node_id: &str) -> Vec<&str> {
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return vec![],
        };
        if let Some(ref s) = node.line {
            return vec![s.as_str()];
        }
        if !node.lines.is_empty() {
            return node.lines.iter().map(String::as_str).collect();
        }
        vec![]
    }

    /// Number of lines (pages) in the node.
    pub fn line_count(&self, node_id: &str) -> usize {
        self.node_lines(node_id).len()
    }

    /// Get the node by id.
    pub fn get_node(&self, node_id: &str) -> Option<&Node> {
        self.nodes.get(node_id)
    }
}

impl Node {
    /// First matching branch for the given story state.
    ///
    /// Supported condition formats: `flag:name`, `path:name`, `quest_active:id`,
    /// `quest_complete:id`.
    pub fn resolve_next(&self, world_state: &crate::state::WorldState) -> Option<String> {
        if let Some(ref next) = self.next {
            return Some(next.clone());
        }
        for branch in &self.branches {
            if branch.matches(world_state) {
                return Some(branch.next.clone());
            }
        }
        None
    }
}

pub(super) fn condition_matches(cond: &str, world_state: &crate::state::WorldState) -> bool {
    let cond = cond.trim();
    if cond.is_empty() {
        return true;
    }
    if let Some(flag) = cond.strip_prefix("flag:") {
        return world_state.has_flag(flag.trim());
    }
    if let Some(path) = cond.strip_prefix("path:") {
        return world_state
            .path()
            .map(|p| p == path.trim())
            .unwrap_or(false);
    }
    if let Some(id) = cond.strip_prefix("quest_active:") {
        return world_state.quest_active(id.trim());
    }
    if let Some(id) = cond.strip_prefix("quest_complete:") {
        return world_state.quest_complete(id.trim());
    }
    log::warn!("dialogue condition has unknown prefix: {:?}", cond);
    false
}

impl Branch {
    fn matches(&self, world_state: &crate::state::WorldState) -> bool {
        let cond = match &self.condition {
            Some(c) => c,
            None => return true,
        };
        condition_matches(cond, world_state)
    }
}

impl Conversation {
    /// Deserialize from JSON string.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        let j: ConversationJson = serde_json::from_str(s)?;
        Ok(Conversation {
            start: j.start,
            nodes: j.nodes,
            require_state: j.require_state,
            default_line: j.default_line,
        })
    }

    /// Build a one-line conversation for backward compat when no conversation file exists.
    pub fn one_line(line: impl Into<String>) -> Self {
        let line = line.into();
        let start = "start".to_string();
        let mut nodes = HashMap::new();
        nodes.insert(
            "start".to_string(),
            Node {
                line: Some(line),
                lines: vec![],
                next: None,
                branches: vec![],
                effects: vec![],
            },
        );
        Conversation {
            start,
            nodes,
            require_state: None,
            default_line: None,
        }
    }
}
