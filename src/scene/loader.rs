//! Load scene definitions from `assets/scenes/{id}.scene.json`.

use std::fmt;
use std::path::PathBuf;

use super::defs::SceneDef;
use crate::paths;

fn scenes_dir() -> PathBuf {
    paths::asset_root().join("scenes")
}

/// Failure while loading or parsing a scene file.
#[derive(Debug)]
pub enum SceneLoadError {
    Io { path: PathBuf, source: std::io::Error },
    Json { path: PathBuf, source: serde_json::Error },
}

impl fmt::Display for SceneLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SceneLoadError::Io { path, source } => {
                write!(f, "scene file read failed ({}): {}", path.display(), source)
            }
            SceneLoadError::Json { path, source } => {
                write!(f, "scene JSON parse failed ({}): {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for SceneLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SceneLoadError::Io { source, .. } => Some(source),
            SceneLoadError::Json { source, .. } => Some(source),
        }
    }
}

/// Load `assets/scenes/{id}.scene.json` and deserialize into [`SceneDef`].
pub fn load(id: &str) -> Result<SceneDef, SceneLoadError> {
    let path = scenes_dir().join(format!("{}.scene.json", id));
    let raw = std::fs::read_to_string(&path).map_err(|source| SceneLoadError::Io {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| SceneLoadError::Json { path, source })
}
