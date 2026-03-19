//! # Story state
//!
//! Global storyline evolution: path (archetype choice) and arbitrary flags.
//! Data-driven: path is a string id so new paths can be added in content without code changes.

use std::collections::HashSet;

/// Global story state. None path = neutral (no archetype chosen yet).
#[derive(Debug, Default)]
pub struct StoryState {
    /// Chosen archetype path, e.g. "bandit", "sheriff", "renegade". None = neutral.
    path: Option<String>,
    /// Arbitrary progress flags, e.g. "met_sheriff", "chose_bandit", "intro_done".
    flags: HashSet<String>,
}

impl StoryState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current path id, or None if neutral.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Commit to a path (e.g. after a decision). Id is any string; no enum, so new paths are data-only.
    pub fn choose_path(&mut self, path_id: impl Into<String>) {
        self.path = Some(path_id.into());
    }

    /// Set a flag (e.g. "met_sheriff", "intro_duel_done").
    pub fn set_flag(&mut self, flag: impl Into<String>) {
        self.flags.insert(flag.into());
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// For dialogue/conditions: check path (and optional flags) without touching Rust when adding content.
    pub fn is_neutral(&self) -> bool {
        self.path.is_none()
    }
}
