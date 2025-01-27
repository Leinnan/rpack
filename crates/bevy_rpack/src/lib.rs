#![doc = include_str!("../README.md")]

#[cfg(feature = "bevy")]
/// Contains the Bevy plugin for handling `Rpack` assets and atlases.
mod plugin;

/// Re-exports all types for working with texture atlases.
pub mod prelude {
    #[cfg(feature = "bevy")]
    /// Provides easy access to `Rpack` asset-related functionality in a Bevy application.
    pub use super::plugin::{
        RpackAssetHelper, RpackAssetPlugin, RpackAtlasAsset, RpackAtlasAssetError,
        RpackAtlasAssetLoader, RpackAtlasError, RpackAtlases,
    };
    /// Re-exports core types for working with texture atlases.
    pub use super::{AtlasAsset, AtlasFrame, SerializableRect};
}

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Reflect))]
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

/// Represents a single frame within a texture atlas, including its identifier and position.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Reflect))]
pub struct AtlasFrame {
    /// A unique identifier for the frame.
    pub key: String,
    /// The rectangular area of the frame within the texture atlas.
    pub frame: SerializableRect,
}

/// Represents an entire texture atlas asset, including its metadata and frames.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Asset, bevy::prelude::Reflect))]
pub struct AtlasAsset {
    /// The overall dimensions of the texture atlas in pixels (width, height).
    pub size: [u32; 2],
    /// The filename associated with the atlas, typically used for loading or debugging.
    pub filename: String,
    /// A collection of frames contained within the texture atlas.
    pub frames: Vec<AtlasFrame>,
}
