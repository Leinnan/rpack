use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewSettings {
    pub display_json: bool,
    pub show_outlines: bool,
    /// Multiplier of the default egui text sizes.
    pub ui_scale: f32,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            display_json: true,
            show_outlines: false,
            ui_scale: 1.4,
        }
    }
}
