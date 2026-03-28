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
use std::fmt;

/// Failure when parsing a condition or effect string (e.g. unknown prefix, empty payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueParseError {
    EmptyInput { kind: &'static str },
    EmptyPayload { prefix: &'static str },
    UnknownPrefix {
        kind: &'static str,
        input: String,
    },
}

impl fmt::Display for DialogueParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DialogueParseError::EmptyInput { kind } => {
                write!(f, "empty {kind} string (expected a prefix like flag:…)")
            }
            DialogueParseError::EmptyPayload { prefix } => {
                write!(f, "empty payload after prefix {prefix:?}")
            }
            DialogueParseError::UnknownPrefix { kind, input } => {
                let expected = match *kind {
                    "condition" => "flag:, path:, quest_active:, quest_complete:",
                    "effect" => "set_flag:, set_path:, start_quest:, complete_quest:",
                    _ => "known prefixes",
                };
                write!(f, "unknown {kind} prefix (expected {expected}): {input:?}")
            }
        }
    }
}

impl std::error::Error for DialogueParseError {}

#[allow(clippy::type_complexity)]
fn parse_prefixed<'a, T>(
    input: &'a str,
    kind: &'static str,
    table: &[(&'static str, fn(&'a str) -> T)],
) -> Result<T, DialogueParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DialogueParseError::EmptyInput { kind });
    }
    for (prefix, build) in table {
        if let Some(rest) = input.strip_prefix(prefix) {
            let payload = rest.trim();
            if payload.is_empty() {
                return Err(DialogueParseError::EmptyPayload { prefix });
            }
            return Ok(build(payload));
        }
    }
    Err(DialogueParseError::UnknownPrefix {
        kind,
        input: input.to_string(),
    })
}

/// Branch / gate condition: same prefixes as documented above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueCondition {
    Flag(String),
    Path(String),
    QuestActive(String),
    QuestComplete(String),
}

impl DialogueCondition {
    pub fn parse(s: &str) -> Result<Self, DialogueParseError> {
        parse_prefixed(
            s,
            "condition",
            &[
                ("flag:", |p| DialogueCondition::Flag(p.to_string())),
                ("path:", |p| DialogueCondition::Path(p.to_string())),
                ("quest_active:", |p| DialogueCondition::QuestActive(p.to_string())),
                ("quest_complete:", |p| DialogueCondition::QuestComplete(p.to_string())),
            ],
        )
    }

    pub fn matches(&self, world_state: &crate::state::WorldState) -> bool {
        match self {
            DialogueCondition::Flag(name) => world_state.has_flag(name),
            DialogueCondition::Path(name) => world_state
                .path()
                .map(|p| p == name.as_str())
                .unwrap_or(false),
            DialogueCondition::QuestActive(id) => world_state.quest_active(id),
            DialogueCondition::QuestComplete(id) => world_state.quest_complete(id),
        }
    }
}

/// Node effect applied after the last line of a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueEffect {
    SetFlag(String),
    SetPath(String),
    StartQuest(String),
    CompleteQuest(String),
}

impl DialogueEffect {
    pub fn parse(s: &str) -> Result<Self, DialogueParseError> {
        parse_prefixed(
            s,
            "effect",
            &[
                ("set_flag:", |p| DialogueEffect::SetFlag(p.to_string())),
                ("set_path:", |p| DialogueEffect::SetPath(p.to_string())),
                ("start_quest:", |p| DialogueEffect::StartQuest(p.to_string())),
                ("complete_quest:", |p| DialogueEffect::CompleteQuest(p.to_string())),
            ],
        )
    }

    pub fn apply(&self, world_state: &mut crate::state::WorldState) {
        match self {
            DialogueEffect::SetFlag(name) => world_state.set_flag(name),
            DialogueEffect::SetPath(name) => world_state.choose_path(name),
            DialogueEffect::StartQuest(id) => world_state.start_quest(id),
            DialogueEffect::CompleteQuest(id) => world_state.complete_quest(id),
        }
    }
}

/// Load-time failure for conversation JSON (serde or dialogue string rules).
#[derive(Debug)]
pub enum ConversationLoadError {
    Json(serde_json::Error),
    Dialogue(String),
}

impl fmt::Display for ConversationLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversationLoadError::Json(e) => write!(f, "{e}"),
            ConversationLoadError::Dialogue(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ConversationLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConversationLoadError::Json(e) => Some(e),
            ConversationLoadError::Dialogue(_) => None,
        }
    }
}

impl From<serde_json::Error> for ConversationLoadError {
    fn from(e: serde_json::Error) -> Self {
        ConversationLoadError::Json(e)
    }
}

/// One conversation: start node id and all nodes by id.
///
/// Optional `require_state` gates entry: if present, [`crate::dialogue::state_satisfied`] must
/// return `true` before the tree opens. When gating fails, `default_line` is shown instead.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub start: String,
    pub nodes: HashMap<String, Node>,
    /// Optional condition that must hold for this conversation to open.
    /// `None` = always open.
    pub require_state: Option<DialogueCondition>,
    /// Line shown when `require_state` is not satisfied. If `None`, dialogue is silently skipped.
    pub default_line: Option<String>,
}

/// One node: one or more lines (JSON may use `line` or `lines`; merged at load); then next node or branches.
#[derive(Debug, Clone)]
pub struct Node {
    pub lines: Vec<String>,
    pub next: Option<String>,
    pub branches: Vec<Branch>,
    pub effects: Vec<DialogueEffect>,
}

/// JSON shape: optional `line` and/or `lines`. Loaded into [`Node`] with `line` taking precedence (same as legacy runtime).
#[derive(Debug, Deserialize)]
struct NodeSerde {
    #[serde(default)]
    line: Option<String>,
    #[serde(default)]
    lines: Vec<String>,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    branches: Vec<BranchSerde>,
    #[serde(default)]
    effects: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BranchSerde {
    #[serde(default)]
    condition: Option<String>,
    next: String,
}

/// Conditional branch: optional condition, next node id.
///
/// A branch with no condition always matches (use as default/fallback after specific conditions).
#[derive(Debug, Clone)]
pub struct Branch {
    pub condition: Option<DialogueCondition>,
    pub next: String,
}

impl Conversation {
    /// Lines for this node (normalized at load). Empty slice if the id is missing.
    pub fn node_lines(&self, node_id: &str) -> &[String] {
        match self.nodes.get(node_id) {
            Some(n) => n.lines.as_slice(),
            None => &[],
        }
    }

    /// Number of lines (pages) in the node.
    pub fn line_count(&self, node_id: &str) -> usize {
        self.nodes.get(node_id).map(|n| n.lines.len()).unwrap_or(0)
    }

    /// Get the node by id.
    pub fn get_node(&self, node_id: &str) -> Option<&Node> {
        self.nodes.get(node_id)
    }
}

impl Node {
    /// First matching branch for the given story state.
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

impl Branch {
    fn matches(&self, world_state: &crate::state::WorldState) -> bool {
        match &self.condition {
            None => true,
            Some(c) => c.matches(world_state),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConversationJson {
    pub start: String,
    pub nodes: HashMap<String, NodeSerde>,
    #[serde(default)]
    pub require_state: Option<String>,
    #[serde(default)]
    pub default_line: Option<String>,
}

fn parse_optional_require_state(
    raw: Option<String>,
) -> Result<Option<DialogueCondition>, ConversationLoadError> {
    let Some(s) = raw else {
        return Ok(None);
    };
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    DialogueCondition::parse(t)
        .map(Some)
        .map_err(|e| ConversationLoadError::Dialogue(format!("require_state: {e} (input: {s:?})")))
}

fn parse_branch(
    node_id: &str,
    index: usize,
    b: BranchSerde,
) -> Result<Branch, ConversationLoadError> {
    let condition = match b.condition {
        None => None,
        Some(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(DialogueCondition::parse(t).map_err(|e| {
                    ConversationLoadError::Dialogue(format!(
                        "nodes[{node_id}].branches[{index}].condition: {e} (input: {s:?})"
                    ))
                })?)
            }
        }
    };
    Ok(Branch {
        condition,
        next: b.next,
    })
}

fn parse_node_effects(node_id: &str, effects: Vec<String>) -> Result<Vec<DialogueEffect>, ConversationLoadError> {
    effects
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            DialogueEffect::parse(s.trim()).map_err(|e| {
                ConversationLoadError::Dialogue(format!(
                    "nodes[{node_id}].effects[{i}]: {e} (input: {s:?})"
                ))
            })
        })
        .collect()
}

impl Conversation {
    /// Deserialize from JSON string. Unknown condition/effect prefixes fail load.
    pub fn from_json(s: &str) -> Result<Self, ConversationLoadError> {
        let j: ConversationJson = serde_json::from_str(s)?;
        let require_state = parse_optional_require_state(j.require_state)?;

        let mut nodes = HashMap::new();
        for (node_id, raw) in j.nodes {
            let lines = if let Some(l) = raw.line {
                vec![l]
            } else {
                raw.lines
            };
            let branches = raw
                .branches
                .into_iter()
                .enumerate()
                .map(|(i, b)| parse_branch(&node_id, i, b))
                .collect::<Result<Vec<_>, _>>()?;
            let effects = parse_node_effects(&node_id, raw.effects)?;
            nodes.insert(
                node_id,
                Node {
                    lines,
                    next: raw.next,
                    branches,
                    effects,
                },
            );
        }

        Ok(Conversation {
            start: j.start,
            nodes,
            require_state,
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
                lines: vec![line],
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
