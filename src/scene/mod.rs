//! Scene module: data types, loader, and load-on-demand cache.
//!
//! Files live at `assets/scenes/{id}.scene.json`. Components store only the
//! scene id string; systems call [`SceneCache::get_or_load`] when they need
//! the full definition (antipattern §2 — no static content in ECS components).

mod defs;
mod loader;

pub use defs::{SceneDef, SceneStep};
pub use loader::SceneLoadError;

use std::collections::HashMap;

/// Load-on-demand cache for scene definitions.
#[derive(Debug, Default)]
pub struct SceneCache {
    cache: HashMap<String, SceneDef>,
}

impl SceneCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cached [`SceneDef`] for `id`, loading from disk on first access.
    pub fn get_or_load(&mut self, id: &str) -> Result<&SceneDef, SceneLoadError> {
        if !self.cache.contains_key(id) {
            let def = loader::load(id)?;
            self.cache.insert(id.to_string(), def);
        }
        Ok(self.cache.get(id).expect("just inserted"))
    }
}
