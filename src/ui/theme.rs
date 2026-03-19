//! UI theme: colors and sizes. Loaded from assets/ui/theme.json or Default.

use serde::Deserialize;

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
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            dialogue_panel_fill: [0.12, 0.1, 0.14],
            dialogue_panel_border: [0.35, 0.32, 0.38],
            dialogue_text: [0.95, 0.9, 0.85],
            dialogue_box_height: 60,
            dialogue_padding_x: 24,
            dialogue_padding_y: 20,
            dialogue_border_top_px: 2,
        }
    }
}

/// Serde-friendly shape for theme.json (nested "dialogue" object).
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct UiThemeFile {
    dialogue: DialogueThemeFile,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DialogueThemeFile {
    panel_fill: [f32; 3],
    panel_border: [f32; 3],
    text: [f32; 3],
    box_height: i32,
    padding_x: i32,
    padding_y: i32,
    border_top_px: i32,
}

impl From<DialogueThemeFile> for UiTheme {
    fn from(f: DialogueThemeFile) -> Self {
        Self {
            dialogue_panel_fill: f.panel_fill,
            dialogue_panel_border: f.panel_border,
            dialogue_text: f.text,
            dialogue_box_height: if f.box_height > 0 { f.box_height } else { 60 },
            dialogue_padding_x: if f.padding_x >= 0 { f.padding_x } else { 24 },
            dialogue_padding_y: if f.padding_y >= 0 { f.padding_y } else { 20 },
            dialogue_border_top_px: if f.border_top_px >= 0 {
                f.border_top_px
            } else {
                2
            },
        }
    }
}

/// Load theme from assets/ui/theme.json. Returns Default on missing or parse error.
pub fn load_theme() -> UiTheme {
    let path = "assets/ui/theme.json";
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return UiTheme::default(),
    };
    let file: UiThemeFile = match serde_json::from_str(&s) {
        Ok(f) => f,
        Err(_) => return UiTheme::default(),
    };
    UiTheme::from(file.dialogue)
}
