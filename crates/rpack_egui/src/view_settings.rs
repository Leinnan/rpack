use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSettings {
    pub preview_max_size: f32,
    pub display_json: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            preview_max_size: 512.0,
            display_json: true,
        }
    }
}
