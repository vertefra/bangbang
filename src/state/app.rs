//! # App state (game mode)
//!
//! **High-level:** The top-level state machine: Overworld, Dialogue, or Duel. Overworld and
//! Dialogue are implemented here; Duel remains a placeholder.

use crate::constants::DIALOGUE_CHARS_PER_SEC;
use crate::dialogue;
use crate::ecs::World;
use crate::map::Tilemap;
use crate::state::{InputState, WorldState};

const MISSING_DIALOGUE_PLACEHOLDER: &str = "...";

/// Current game mode. **Rust:** An `enum` is a type that is exactly one of its variants; we'll match on it for mode-specific behaviour.
/// `Overworld { last_near_npc }` tracks if player was near an NPC last frame so we only trigger dialogue when entering range, not when already standing there (e.g. after closing).
/// `Dialogue` holds conversation state; dialogue module resolves current line and advance.
#[derive(Debug, Clone, PartialEq)]
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
        /// Unicode scalar values (`char`) of the current line already revealed (typewriter).
        stream_visible: u32,
        stream_acc: f32,
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
            AppState::Overworld {
                last_near_npc,
                backpack_open,
            } => {
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
                            let npc_id = interaction.npc_id;
                            let conversation_id = interaction.conversation_id;
                            let open_state = if let Some(conv) =
                                dialogue_cache.get_or_load(&conversation_id)
                            {
                                match dialogue::entry_point(conv, world_state) {
                                    Some((node_id, line_index)) => Some((
                                        conversation_id.clone(),
                                        node_id,
                                        line_index,
                                    )),
                                    None => conv.default_line.clone().and_then(|default_line| {
                                        let fallback_id =
                                            format!("{}_fallback", conversation_id);
                                        let temp_conv =
                                            dialogue::Conversation::one_line(&default_line);
                                        let (node_id, line_index) =
                                            dialogue::entry_point(&temp_conv, world_state)?;
                                        dialogue_cache
                                            .insert_generated(fallback_id.clone(), temp_conv);
                                        Some((fallback_id, node_id, line_index))
                                    }),
                                }
                            } else {
                                let fallback_id = format!("{}_missing", conversation_id);
                                let temp_conv = dialogue::Conversation::one_line(
                                    MISSING_DIALOGUE_PLACEHOLDER,
                                );
                                let (node_id, line_index) =
                                    dialogue::entry_point(&temp_conv, world_state)
                                        .expect("one-line fallback must have an entry point");
                                dialogue_cache.insert_generated(fallback_id.clone(), temp_conv);
                                log::warn!(
                                    "dialogue file missing for {}; using placeholder conversation",
                                    conversation_id
                                );
                                Some((fallback_id, node_id, line_index))
                            };

                            if let Some((conversation_id, node_id, line_index)) = open_state {
                                *self = AppState::Dialogue {
                                    npc_id,
                                    conversation_id,
                                    node_id,
                                    line_index,
                                    stream_visible: 0,
                                    stream_acc: 0.0,
                                };
                                return;
                            }
                            log::warn!(
                                "dialogue open skipped (no line to show, no default line) for {}",
                                conversation_id
                            );
                        }
                    }
                    *last_near_npc = near_now;
                }
            }
            AppState::Dialogue {
                conversation_id,
                node_id,
                line_index,
                stream_visible,
                stream_acc,
                ..
            } => {
                let Some(conv) = dialogue_cache.get_or_load(conversation_id) else {
                    log::warn!(
                        "dialogue state referenced uncached or missing conversation {}; closing",
                        conversation_id
                    );
                    *self = AppState::Overworld {
                        last_near_npc: true,
                        backpack_open: false,
                    };
                    return;
                };
                let full_line = dialogue::current_display(conv, node_id, *line_index);
                let len = full_line.map(|s| s.chars().count() as u32).unwrap_or(0);

                if len > 0 && *stream_visible < len {
                    *stream_acc += dt * DIALOGUE_CHARS_PER_SEC;
                    while *stream_visible < len && *stream_acc >= 1.0 {
                        *stream_acc -= 1.0;
                        *stream_visible += 1;
                    }
                }

                if input.confirm_pressed {
                    input.confirm_pressed = false;
                    if *stream_visible < len {
                        *stream_visible = len;
                        *stream_acc = 0.0;
                    } else {
                        let result = dialogue::advance(conv, node_id, *line_index, world_state);
                        if result.finished {
                            *self = AppState::Overworld {
                                last_near_npc: true,
                                backpack_open: false,
                            };
                        } else {
                            *node_id = result.node_id;
                            *line_index = result.line_index;
                            *stream_visible = 0;
                            *stream_acc = 0.0;
                        }
                    }
                }
            }
            AppState::Duel => {}
        }
    }

    /// Full current line (for debugging or callers that need the complete text).
    pub fn dialogue_message(
        &self,
        dialogue_cache: &mut dialogue::ConversationCache,
    ) -> Option<String> {
        match self {
            AppState::Dialogue {
                conversation_id,
                node_id,
                line_index,
                ..
            } => dialogue_cache
                .get_or_load(conversation_id)
                .and_then(|conv| dialogue::current_display(conv, node_id, *line_index))
                .map(String::from),
            _ => None,
        }
    }

    /// Text to draw: current line truncated to the typewriter-visible prefix.
    pub fn dialogue_display_text(
        &self,
        dialogue_cache: &mut dialogue::ConversationCache,
    ) -> Option<String> {
        match self {
            AppState::Dialogue {
                conversation_id,
                node_id,
                line_index,
                stream_visible,
                ..
            } => dialogue_cache
                .get_or_load(conversation_id)
                .and_then(|conv| dialogue::current_display(conv, node_id, *line_index))
                .map(|full| {
                    full.chars()
                        .take(*stream_visible as usize)
                        .collect::<String>()
                }),
            _ => None,
        }
    }
}
