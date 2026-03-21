//! Skill definitions deserialized from `assets/skills/{id}.json`.

use serde::Deserialize;

/// Top-level category: permanent (e.g. weapon passive) vs usable (consumable with charges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    Permanent,
    Usable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOp {
    DealDamage,
    Heal,
}

/// Who receives this effect step. Resolved at runtime from duel/overworld context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectTarget {
    Caster,
    Opponent,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectStep {
    pub op: EffectOp,
    pub target: EffectTarget,
    pub amount: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub category: SkillCategory,
    #[serde(default)]
    pub subcategory: String,
    /// Initial charges when granting this usable skill (only meaningful for `Usable`).
    #[serde(default)]
    pub charges_default: Option<u32>,
    #[serde(default)]
    pub effects: Vec<EffectStep>,
}

impl SkillDef {
    /// Load one skill from `assets/skills/{id}.json`.
    pub fn load(id: &str) -> Result<Self, String> {
        let path = crate::paths::asset_root()
            .join("skills")
            .join(format!("{}.json", id));
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("skills: read {}: {}", path.display(), e))?;
        let def: SkillDef = serde_json::from_str(&raw)
            .map_err(|e| format!("skills: parse {}: {}", path.display(), e))?;
        if def.id != id {
            return Err(format!(
                "skills: file {} has id {:?}, expected {:?}",
                path.display(),
                def.id,
                id
            ));
        }
        if def.effects.is_empty() {
            return Err(format!("skills: {} has no effects", id));
        }
        for step in &def.effects {
            if step.amount < 0 {
                return Err(format!(
                    "skills: {} effect {:?} has negative amount",
                    id, step.op
                ));
            }
        }
        match def.category {
            SkillCategory::Usable if def.charges_default.is_none() || def.charges_default == Some(0) => {
                return Err(format!(
                    "skills: usable {} must set charges_default > 0",
                    id
                ));
            }
            _ => {}
        }
        Ok(def)
    }
}
