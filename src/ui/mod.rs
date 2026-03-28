//! # UI module
//!
//! Data models, theme, and layout math for the game UI. All actual draw calls happen in
//! `gpu/renderer.rs`; this module prepares the data they consume. See `docs/ui.md`.

pub mod backpack;
pub mod layout;
pub mod theme;

pub use backpack::{backpack_panel_lines, BackpackPanelLines};
pub use layout::*;
pub use theme::*;
