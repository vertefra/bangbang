//! In-memory registry of loaded [`SkillDef`]s.

use std::collections::HashMap;

use super::defs::SkillDef;

#[derive(Debug, Clone)]
pub struct SkillRegistry {
    defs: HashMap<String, SkillDef>,
}

impl SkillRegistry {
    /// Load all skills found in the `assets/skills/` directory.
    pub fn load_builtins() -> Result<Self, String> {
        let skills_dir = crate::paths::asset_root().join("skills");
        let mut defs = HashMap::new();
        
        match std::fs::read_dir(&skills_dir) {
            Ok(entries) => {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "json") {
                        if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                            let def = SkillDef::load(id)?;
                            defs.insert(id.to_string(), def);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("failed to read skills directory {}: {}", skills_dir.display(), e);
            }
        }
        
        if defs.is_empty() {
             log::warn!("no skills loaded from {}", skills_dir.display());
        }

        Ok(Self { defs })
    }

    pub fn get(&self, id: &str) -> Option<&SkillDef> {
        self.defs.get(id)
    }
}
