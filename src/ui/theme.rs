//! UI theme: colors and sizes loaded from `assets/ui/theme.json`.

use serde::Deserialize;
use std::path::PathBuf;

/// Colors and dimensions for all UI. Data-driven via theme.json; Default matches legacy hardcoded values.
#[derive(Debug, Clone)]
pub struct UiTheme {
    // Dialogue box
    pub dialogue_panel_fill: [f32; 3],
    pub dialogue_panel_border: [f32; 3],
    pub dialogue_text: [f32; 3],
    pub dialogue_box_height: i32,
    pub dialogue_padding_x: i32,
    pub dialogue_padding_y: i32,
    pub dialogue_border_top_px: i32,
    pub dialogue_portrait_width: i32,
    pub dialogue_portrait_height: i32,
    pub dialogue_portrait_gap: i32,
    // Backpack
    pub backpack_panel_fill: [f32; 3],
    pub backpack_panel_border: [f32; 3],
    pub backpack_slot_empty: [f32; 3],
    pub backpack_panel_width: i32,
    pub backpack_panel_height: i32,
    pub backpack_padding: i32,
    pub backpack_slot_height: i32,
    pub backpack_border_top_px: i32,
    pub backpack_section_usable: [f32; 3],
    pub backpack_section_weapon: [f32; 3],
    pub backpack_section_passive: [f32; 3],
    pub backpack_row_weapon: [f32; 3],
    pub backpack_row_passive: [f32; 3],
    pub backpack_row_equipped: [f32; 3],
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            dialogue_panel_fill: [0.12, 0.1, 0.14],
            dialogue_panel_border: [0.35, 0.32, 0.38],
            dialogue_text: [0.95, 0.9, 0.85],
            dialogue_box_height: 100,
            dialogue_padding_x: 24,
            dialogue_padding_y: 20,
            dialogue_border_top_px: 2,
            dialogue_portrait_width: 64,
            dialogue_portrait_height: 64,
            dialogue_portrait_gap: 12,
            backpack_panel_fill: [0.1, 0.08, 0.12],
            backpack_panel_border: [0.3, 0.28, 0.34],
            backpack_slot_empty: [0.5, 0.48, 0.52],
            backpack_panel_width: 320,
            backpack_panel_height: 560,
            backpack_padding: 16,
            backpack_slot_height: 24,
            backpack_border_top_px: 2,
            backpack_section_usable: [0.9, 0.85, 0.75],
            backpack_section_weapon: [0.85, 0.82, 0.72],
            backpack_section_passive: [0.78, 0.76, 0.82],
            backpack_row_weapon: [0.88, 0.84, 0.78],
            backpack_row_passive: [0.72, 0.68, 0.76],
            backpack_row_equipped: [0.95, 0.42, 0.32],
        }
    }
}

/// Serde-friendly shape for theme.json (nested "dialogue" and "backpack" objects).
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct UiThemeFile {
    dialogue: DialogueThemeFile,
    backpack: BackpackThemeFile,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DialogueThemeFile {
    panel_fill: Option<[f32; 3]>,
    panel_border: Option<[f32; 3]>,
    text: Option<[f32; 3]>,
    box_height: Option<i32>,
    padding_x: Option<i32>,
    padding_y: Option<i32>,
    border_top_px: Option<i32>,
    portrait_width: Option<i32>,
    portrait_height: Option<i32>,
    portrait_gap: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct BackpackThemeFile {
    panel_fill: Option<[f32; 3]>,
    panel_border: Option<[f32; 3]>,
    slot_empty: Option<[f32; 3]>,
    panel_width: Option<i32>,
    panel_height: Option<i32>,
    padding: Option<i32>,
    slot_height: Option<i32>,
    border_top_px: Option<i32>,
    section_usable: Option<[f32; 3]>,
    section_weapon: Option<[f32; 3]>,
    section_passive: Option<[f32; 3]>,
    row_weapon: Option<[f32; 3]>,
    row_passive: Option<[f32; 3]>,
    row_equipped: Option<[f32; 3]>,
}

impl From<DialogueThemeFile> for UiTheme {
    fn from(f: DialogueThemeFile) -> Self {
        let d = UiTheme::default();
        Self {
            dialogue_panel_fill: f.panel_fill.unwrap_or(d.dialogue_panel_fill),
            dialogue_panel_border: f.panel_border.unwrap_or(d.dialogue_panel_border),
            dialogue_text: f.text.unwrap_or(d.dialogue_text),
            dialogue_box_height: f.box_height.unwrap_or(d.dialogue_box_height),
            dialogue_padding_x: f.padding_x.unwrap_or(d.dialogue_padding_x),
            dialogue_padding_y: f.padding_y.unwrap_or(d.dialogue_padding_y),
            dialogue_border_top_px: f.border_top_px.unwrap_or(d.dialogue_border_top_px),
            dialogue_portrait_width: f.portrait_width.unwrap_or(d.dialogue_portrait_width),
            dialogue_portrait_height: f.portrait_height.unwrap_or(d.dialogue_portrait_height),
            dialogue_portrait_gap: f.portrait_gap.unwrap_or(d.dialogue_portrait_gap),
            ..d
        }
    }
}

impl UiTheme {
    fn merge_backpack(&mut self, f: BackpackThemeFile) {
        if let Some(v) = f.panel_width {
            self.backpack_panel_width = v;
        }
        if let Some(v) = f.panel_height {
            self.backpack_panel_height = v;
        }
        if let Some(v) = f.padding {
            self.backpack_padding = v;
        }
        if let Some(v) = f.slot_height {
            self.backpack_slot_height = v;
        }
        if let Some(v) = f.border_top_px {
            self.backpack_border_top_px = v;
        }
        if let Some(v) = f.panel_fill {
            self.backpack_panel_fill = v;
        }
        if let Some(v) = f.panel_border {
            self.backpack_panel_border = v;
        }
        if let Some(v) = f.slot_empty {
            self.backpack_slot_empty = v;
        }
        if let Some(v) = f.section_usable {
            self.backpack_section_usable = v;
        }
        if let Some(v) = f.section_weapon {
            self.backpack_section_weapon = v;
        }
        if let Some(v) = f.section_passive {
            self.backpack_section_passive = v;
        }
        if let Some(v) = f.row_weapon {
            self.backpack_row_weapon = v;
        }
        if let Some(v) = f.row_passive {
            self.backpack_row_passive = v;
        }
        if let Some(v) = f.row_equipped {
            self.backpack_row_equipped = v;
        }
    }
}

#[derive(Debug)]
pub enum ThemeLoadError {
    Io(std::io::Error, PathBuf),
    Json(serde_json::Error, PathBuf),
}

impl std::fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e, p) => write!(f, "IO error at {}: {}", p.display(), e),
            Self::Json(e, p) => write!(f, "JSON error at {}: {}", p.display(), e),
        }
    }
}

/// Load theme from assets/ui/theme.json.
pub fn load_theme() -> Result<UiTheme, ThemeLoadError> {
    let path = crate::paths::asset_root().join("ui/theme.json");
    let s = std::fs::read_to_string(&path).map_err(|e| ThemeLoadError::Io(e, path.clone()))?;
    let file: UiThemeFile =
        serde_json::from_str(&s).map_err(|e| ThemeLoadError::Json(e, path.clone()))?;
    let mut theme = UiTheme::from(file.dialogue);
    theme.merge_backpack(file.backpack);
    Ok(theme)
}
