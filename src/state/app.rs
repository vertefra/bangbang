//! # App state (game mode)
//!
//! **High-level:** The top-level state machine: Overworld, Dialogue, or Duel. Currently only
//! Overworld is implemented; `update` delegates to overworld logic. Later, `update`/draw will
//! branch on `self` to run mode-specific logic.

use crate::dialogue;
use crate::ecs::World;
use crate::map::Tilemap;
use crate::state::{InputState, StoryState};

/// Current game mode. **Rust:** An `enum` is a type that is exactly one of its variants; we'll match on it for mode-specific behaviour.
/// `Overworld { last_near_npc }` tracks if player was near an NPC last frame so we only trigger dialogue when entering range, not when already standing there (e.g. after closing).
/// `Dialogue` holds conversation state; dialogue module resolves current line and advance.
#[derive(Debug)]
pub enum AppState {
    Overworld { last_near_npc: bool },
    Dialogue {
        npc_id: String,
        conversation_id: String,
        fallback_line: String,
        node_id: String,
        line_index: u32,
    },
    Duel,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Overworld { last_near_npc: false }
    }
}

impl AppState {
    /// One tick of game logic. Branches on state: Overworld (movement + maybe trigger dialogue), Dialogue (confirm to close), Duel (no-op).
    /// Story state is passed so dialogue (and later choices) can set path or flags.
    pub fn update(
        &mut self,
        world: &mut World,
        input: &mut InputState,
        _story: &mut StoryState,
        dt: f32,
        tilemap: &Tilemap,
    ) {
        match self {
            AppState::Overworld { last_near_npc } => {
                let (trigger, near_now) = super::overworld::update(world, input, dt, tilemap);
                if !*last_near_npc {
                    if let Some((npc_id, conversation_id, fallback_line)) = trigger {
                        let conv = dialogue::load_or_fallback(&conversation_id, &fallback_line);
                        *self = AppState::Dialogue {
                            npc_id,
                            conversation_id,
                            fallback_line,
                            node_id: conv.start.clone(),
                            line_index: 0,
                        };
                    } else {
                        *self = AppState::Overworld { last_near_npc: near_now };
                    }
                } else {
                    *self = AppState::Overworld { last_near_npc: near_now };
                }
            }
            AppState::Dialogue {
                conversation_id,
                fallback_line,
                node_id,
                line_index,
                ..
            } => {
                if input.confirm_pressed {
                    input.confirm_pressed = false;
                    let conv = dialogue::load_or_fallback(conversation_id, fallback_line);
                    let result = dialogue::advance(&conv, node_id, *line_index, _story);
                    if result.finished {
                        *self = AppState::Overworld { last_near_npc: true };
                    } else {
                        *node_id = result.node_id;
                        *line_index = result.line_index;
                    }
                }
            }
            AppState::Duel => {}
        }
    }

    /// Reference to the current dialogue message if in Dialogue state, for drawing.
    /// Resolves from dialogue module so conversation trees and paging are respected.
    pub fn dialogue_message(&self) -> Option<String> {
        match self {
            AppState::Dialogue {
                conversation_id,
                fallback_line,
                node_id,
                line_index,
                ..
            } => {
                let conv = dialogue::load_or_fallback(conversation_id, fallback_line);
                dialogue::current_display(&conv, node_id, *line_index).map(String::from)
            }
            _ => None,
        }
    }
}
