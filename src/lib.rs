//! # BangBang library root
//!
//! **High-level:** This file is the crate's public API surface. It declares which modules exist
//! and re-exports them so that `main.rs` (or other binaries) can use `bangbang::ecs`, etc.
//!
//! ## Rust: `mod` and visibility
//! - `pub mod name` = "this crate has a submodule named `name`" and that module is **public**
//!   (other crates can use it via `bangbang::ecs`).
//! - Without `pub`, the module would be private to this crate only.

pub mod assets;
pub mod config;
pub mod constants;
pub mod dialogue;
pub mod ecs;
pub mod gpu;
pub mod map;
pub mod map_loader;
pub mod paths;
pub mod render;
pub mod render_settings;
pub mod save_game;
pub mod skills;
pub mod state;
pub mod ui;
