//! Conversation tree types: nodes, branches, effects. Deserialized from JSON.

use serde::Deserialize;
use std::collections::HashMap;

/// One conversation: start node id and all nodes by id.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub start: String,
    pub nodes: HashMap<String, Node>,
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

/// Conditional branch: optional condition (e.g. "flag:met_sheriff", "path:bandit"), next node id.
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
    /// First matching branch for the given story state. Condition format: "flag:name" or "path:name".
    pub fn resolve_next(&self, story: &crate::state::StoryState) -> Option<String> {
        if let Some(ref next) = self.next {
            return Some(next.clone());
        }
        for branch in &self.branches {
            if branch.matches(story) {
                return Some(branch.next.clone());
            }
        }
        None
    }
}

impl Branch {
    fn matches(&self, story: &crate::state::StoryState) -> bool {
        let cond = match &self.condition {
            Some(c) => c,
            None => return true,
        };
        let cond = cond.trim();
        if cond.is_empty() {
            return true;
        }
        if let Some(flag) = cond.strip_prefix("flag:") {
            return story.has_flag(flag.trim());
        }
        if let Some(path) = cond.strip_prefix("path:") {
            return story
                .path()
                .map(|p| p == path.trim())
                .unwrap_or(false);
        }
        false
    }
}

impl Conversation {
    /// Deserialize from JSON string.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        let j: ConversationJson = serde_json::from_str(s)?;
        Ok(Conversation {
            start: j.start,
            nodes: j.nodes,
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
        Conversation { start, nodes }
    }
}
