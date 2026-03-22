//! Load `assets/config.json`: world zoom and UI scale.

use serde::Deserialize;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    crate::paths::asset_root().join("config.json")
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct RenderSettingsFile {
    render_scale: f32,
    ui_scale: u32,
    window_width: u32,
    window_height: u32,
    /// Multiplier on UI glyph size; layout rects still use `ui_scale` only.
    font_scale: Option<f32>,
}

impl Default for RenderSettingsFile {
    fn default() -> Self {
        Self {
            render_scale: 2.0,
            ui_scale: 2,
            window_width: 800,
            window_height: 600,
            font_scale: None,
        }
    }
}

/// World projection scale and UI scale; optional default window size.
#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub render_scale: f32,
    pub ui_scale: u32,
    pub window_width: u32,
    pub window_height: u32,
    pub font_scale: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            render_scale: 2.0,
            ui_scale: 2,
            window_width: 800,
            window_height: 600,
            font_scale: 1.0,
        }
    }
}

impl From<RenderSettingsFile> for RenderSettings {
    fn from(f: RenderSettingsFile) -> Self {
        let render_scale = if f.render_scale > 0.0 {
            f.render_scale
        } else {
            2.0
        };
        let ui_scale = f.ui_scale.max(1);
        let window_width = f.window_width.max(1);
        let window_height = f.window_height.max(1);
        let font_scale = f
            .font_scale
            .filter(|&x| x.is_finite() && x > 0.0)
            .map(|x| x.clamp(0.25, 4.0))
            .unwrap_or(1.0);
        Self {
            render_scale,
            ui_scale,
            window_width,
            window_height,
            font_scale,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error, PathBuf),
    Json(serde_json::Error, PathBuf),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e, p) => write!(f, "IO error at {}: {}", p.display(), e),
            Self::Json(e, p) => write!(f, "JSON error at {}: {}", p.display(), e),
        }
    }
}

/// Load from `assets/config.json`.
pub fn load() -> Result<RenderSettings, ConfigError> {
    let path = config_path();
    let s = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io(e, path.clone()))?;
    let file: RenderSettingsFile =
        serde_json::from_str(&s).map_err(|e| ConfigError::Json(e, path.clone()))?;
    Ok(RenderSettings::from(file))
}
