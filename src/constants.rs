/// Maximum world-unit distance between the player and an NPC for interaction (dialogue trigger,
/// weapon target). Used by `state::overworld` and `skills::apply`. Tune this when sprite sizes
/// or tile sizes change to keep the "talk" and "shoot" radii feeling natural.
pub const NPC_INTERACT_RANGE: f32 = 23.0;

/// Typewriter speed for dialogue (Unicode scalar values per second).
pub const DIALOGUE_CHARS_PER_SEC: f32 = 36.0;

/// Seconds after a map transition before door overlap is evaluated again (avoids instant bounce).
pub const DOOR_TRANSITION_COOLDOWN_SECS: f32 = 0.35;

/// How long a transient overworld message (e.g. blocked door) stays visible.
pub const OVERWORLD_TOAST_DURATION_SECS: f32 = 4.0;

/// Cooldown after a scene finishes (or map transition) before scene proximity triggers fire again.
/// Prevents a scene from immediately re-triggering when the player returns to the trigger area.
pub const SCENE_TRIGGER_COOLDOWN_SECS: f32 = 1.0;

/// Multiplier on `CharacterNpcConfig.scale` for [`crate::ecs::SceneActor`] cutscene sprites (config is often ~0.5; this keeps them readable on screen).
pub const SCENE_ACTOR_SCALE_MULTIPLIER: f32 = 2.25;
