//! # App state (game mode)
//!
//! **High-level:** The top-level state machine: Overworld, Dialogue, or Duel. Currently only
//! Overworld is implemented; `update` delegates to overworld logic. Later, `update`/draw will
//! branch on `self` to run mode-specific logic.

use crate::dialogue;
use crate::ecs::World;
use crate::map::Tilemap;
use crate::state::{InputState, WorldState};

/// Current game mode. **Rust:** An `enum` is a type that is exactly one of its variants; we'll match on it for mode-specific behaviour.
/// `Overworld { last_near_npc }` tracks if player was near an NPC last frame so we only trigger dialogue when entering range, not when already standing there (e.g. after closing).
/// `Dialogue` holds conversation state; dialogue module resolves current line and advance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Overworld { 
        last_near_npc: bool,
        backpack_open: bool,
    },
    Dialogue {
        npc_id: String,
        conversation_id: String,
        node_id: String,
        line_index: u32,
    },
    Duel,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Overworld { 
            last_near_npc: false,
            backpack_open: false,
        }
    }
}

impl AppState {
    /// One tick of game logic. Branches on state: Overworld (movement + maybe trigger dialogue), Dialogue (confirm to close), Duel (no-op).
    /// Story state is passed so dialogue (and later choices) can set path or flags.
    pub fn update(
        &mut self,
        world: &mut World,
        input: &mut InputState,
        world_state: &mut WorldState,
        dt: f32,
        tilemap: &Tilemap,
        skill_registry: &crate::skills::SkillRegistry,
        dialogue_cache: &mut dialogue::ConversationCache,
    ) {
        match self {
            AppState::Overworld { last_near_npc, backpack_open } => {
                if input.backpack_pressed {
                    *backpack_open = !*backpack_open;
                    input.backpack_pressed = false;
                }

                if *backpack_open {
                    if let Some(p) = crate::skills::player_entity(world) {
                        if let Ok(mut b) = world.get::<&mut crate::ecs::Backpack>(p) {
                            crate::skills::normalize_equipped_weapon(&mut b, skill_registry);
                        }
                    }
                    if let Some(d) = input.take_skill_hotkey_digit() {
                        crate::skills::apply_backpack_hotkey(world, skill_registry, d);
                    }
                    if let Some(step) = input.take_weapon_cycle_step() {
                        crate::skills::cycle_equipped_weapon(world, skill_registry, step);
                    }
                } else {
                    let (trigger, near_now) = super::overworld::update(world, input, dt, tilemap);
                    if !*last_near_npc {
                        if let Some(interaction) = trigger {
                            let conv = dialogue_cache.get_or_load_fallback(&interaction.conversation_id, "...");
                            *self = AppState::Dialogue {
                                npc_id: interaction.npc_id,
                                conversation_id: interaction.conversation_id,
                                node_id: conv.start.clone(),
                                line_index: 0,
                            };
                            return;
                        }
                    }
                    *last_near_npc = near_now;
                }
            }
            AppState::Dialogue {
                conversation_id,
                node_id,
                line_index,
                ..
            } => {
                if input.confirm_pressed {
                    input.confirm_pressed = false;
                    let conv = dialogue_cache.get_or_load_fallback(conversation_id, "...");
                    let result = dialogue::advance(conv, node_id, *line_index, world_state);
                    if result.finished {
                        *self = AppState::Overworld { 
                            last_near_npc: true,
                            backpack_open: false,
                        };
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
    pub fn dialogue_message(&self, dialogue_cache: &mut dialogue::ConversationCache) -> Option<String> {
        match self {
            AppState::Dialogue {
                conversation_id,
                node_id,
                line_index,
                ..
            } => {
                let conv = dialogue_cache.get_or_load_fallback(conversation_id, "...");
                dialogue::current_display(conv, node_id, *line_index).map(String::from)
            }
            _ => None,
        }
    }
}
