//! # World state (story / quest tracking)
//!
//! Persistent player choices that outlive individual maps and dialogues. Drives branch
//! conditions in conversations and door `require_state` gates. See `docs/npc.md` (Dialogue)
//! and `docs/maps.md` (doors.json).

/// Global story / quest state. Drives dialogue conditions, quest tracking, and archetype path.
///
/// ## Flags
/// Simple boolean switches (e.g. `"met_sheriff"`, `"intro_done"`). Set via dialogue effects
/// (`set_flag:name`) and checked via conditions (`flag:name`).
///
/// ## Quests
/// Tracked objectives with lifecycle: **active → completed**. Set via dialogue effects
/// (`start_quest:id`, `complete_quest:id`) and checked via conditions (`quest_active:id`,
/// `quest_complete:id`). Unlike flags, quests have distinct active/completed states so gameplay
/// systems can react appropriately (e.g. show an objective marker for active quests, unlock a
/// reward on completion).
///
/// ## Path
/// Chosen archetype (`"bandit"`, `"sheriff"`, `"renegade"`). `None` = neutral (no choice yet).
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Serializable copy of [`WorldState`] for save files (see `save_game` module).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    pub path: Option<String>,
    pub flags: Vec<String>,
    pub active_quests: Vec<String>,
    pub completed_quests: Vec<String>,
}

#[derive(Debug, Default)]
pub struct WorldState {
    /// Chosen archetype path, e.g. "bandit", "sheriff", "renegade". None = neutral.
    path: Option<String>,
    /// Arbitrary progress flags, e.g. "met_sheriff", "chose_bandit", "intro_done".
    flags: HashSet<String>,
    /// Currently active (in-progress) quests, e.g. "withdraw_gold".
    active_quests: HashSet<String>,
    /// Completed quests. Once completed, a quest is no longer active.
    completed_quests: HashSet<String>,
}

impl WorldState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_snapshot(&self) -> WorldStateSnapshot {
        WorldStateSnapshot {
            path: self.path.clone(),
            flags: self.flags.iter().cloned().collect(),
            active_quests: self.active_quests.iter().cloned().collect(),
            completed_quests: self.completed_quests.iter().cloned().collect(),
        }
    }

    /// Replace all story state from a save (used after load).
    pub fn restore_from_snapshot(&mut self, s: WorldStateSnapshot) {
        self.path = s.path;
        self.flags = s.flags.into_iter().collect();
        self.active_quests = s.active_quests.into_iter().collect();
        self.completed_quests = s.completed_quests.into_iter().collect();
    }

    // ── Path ──

    /// Current path id, or None if neutral.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Commit to a path (e.g. after a decision). Id is any string; no enum, so new paths are data-only.
    pub fn choose_path(&mut self, path_id: impl Into<String>) {
        self.path = Some(path_id.into());
    }

    /// For dialogue/conditions: check path (and optional flags) without touching Rust when adding content.
    pub fn is_neutral(&self) -> bool {
        self.path.is_none()
    }

    // ── Flags ──

    /// Set a flag (e.g. "met_sheriff", "intro_duel_done").
    pub fn set_flag(&mut self, flag: impl Into<String>) {
        self.flags.insert(flag.into());
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    // ── Quests ──

    /// Begin tracking a quest. Has no effect if the quest is already active or completed.
    pub fn start_quest(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.completed_quests.contains(&id) {
            self.active_quests.insert(id);
        }
    }

    /// Mark a quest as completed (removes from active).
    pub fn complete_quest(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.active_quests.remove(&id);
        self.completed_quests.insert(id);
    }

    /// True if the quest is currently in progress.
    pub fn quest_active(&self, id: &str) -> bool {
        self.active_quests.contains(id)
    }

    /// True if the quest has been completed.
    pub fn quest_complete(&self, id: &str) -> bool {
        self.completed_quests.contains(id)
    }
}
