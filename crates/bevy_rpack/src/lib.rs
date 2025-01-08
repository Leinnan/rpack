#[cfg(feature = "bevy")]
mod bevy;

pub mod prelude {
    pub use super::{AtlasAsset,SerializableRect, AtlasFrame};
    #[cfg(feature = "bevy")]
    pub use super::bevy::{RpackAssetPlugin, RpackAtlasAsset, RpackAtlasAssetError, RpackAtlasAssetLoader};
}

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializableRect {
    /// Horizontal position the rectangle begins at.
    pub x: u32,
    /// Vertical position the rectangle begins at.
    pub y: u32,
    /// Width of the rectangle.
    pub w: u32,
    /// Height of the rectangle.
    pub h: u32,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AtlasFrame {
    pub key: String,
    pub frame: SerializableRect,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AtlasAsset {
    pub size: [u32; 2],
    pub filename: String,
    pub frames: Vec<AtlasFrame>,
}
