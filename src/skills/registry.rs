//! In-memory registry of loaded [`SkillDef`]s.

use std::collections::HashMap;

use super::defs::SkillDef;

#[derive(Debug, Clone)]
pub struct SkillRegistry {
    defs: HashMap<String, SkillDef>,
}

impl SkillRegistry {
    /// Load all skills found under `assets/skills/` as folders named `{id}.skill/` with `config.json`.
    ///
    /// Fails if the directory cannot be read or if no skill definitions were loaded.
    /// **Weapons** are permanent skills with `subcategory == "weapon"` — same folder layout as other skills.
    pub fn load_builtins() -> Result<Self, String> {
        let skills_dir = crate::paths::asset_root().join("skills");
        let entries = std::fs::read_dir(&skills_dir).map_err(|e| {
            format!(
                "failed to read skills directory {}: {}",
                skills_dir.display(),
                e
            )
        })?;

        let mut defs = HashMap::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                format!(
                    "failed to read entry in skills directory {}: {}",
                    skills_dir.display(),
                    e
                )
            })?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(id) = name.strip_suffix(".skill") else {
                continue;
            };
            let def = SkillDef::load(id)?;
            defs.insert(id.to_string(), def);
        }

        if defs.is_empty() {
            return Err(format!(
                "no skill definitions loaded from {} (expected at least one `<id>.skill/config.json`)",
                skills_dir.display()
            ));
        }

        Ok(Self { defs })
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.defs.contains_key(id)
    }

    /// Iterate `(id, def)` pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SkillDef)> + '_ {
        self.defs.iter().map(|(id, def)| (id.as_str(), def))
    }

    /// Iterate skill ids in arbitrary order.
    pub fn ids(&self) -> impl Iterator<Item = &str> + '_ {
        self.defs.keys().map(|s| s.as_str())
    }

    pub fn get(&self, id: &str) -> Option<&SkillDef> {
        self.defs.get(id)
    }
}
