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
