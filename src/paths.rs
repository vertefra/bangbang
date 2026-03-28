//! # Asset paths
//!
//! Centralized I/O root for all asset loading. Every subsystem resolves files relative to
//! [`asset_root()`] instead of hardcoding paths (see `docs/antipatterns.md` §6). This is
//! the **single** location that uses `env!(CARGO_MANIFEST_DIR)` — keeping it here means
//! only one file changes when the binary packaging or distribution layout changes.

use std::path::PathBuf;

/// Compile-time path to `assets/` under the crate root.
pub fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// `assets/skills/{id}.skill/` — definition (`config.json`) and optional `skill_image.png`.
pub fn skill_asset_dir(id: &str) -> PathBuf {
    asset_root().join("skills").join(format!("{id}.skill"))
}

/// Single save slot: `~/.local/share/bangbang/save.json` when `HOME` is set (Unix), else `bangbang_save.json` in the current working directory.
pub fn save_game_file() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share/bangbang/save.json")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("bangbang_save.json")
    }
}
