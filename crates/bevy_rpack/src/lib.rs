#![doc = include_str!("../README.md")]
extern crate alloc;
use alloc::borrow::Cow;
#[cfg(feature = "bevy")]
use bevy_asset::Asset;
#[cfg(feature = "bevy")]
use bevy_reflect::Reflect;

#[cfg(feature = "bevy")]
/// Contains the Bevy plugin for handling `Rpack` assets and atlases.
mod plugin;

/// Re-exports all types for working with texture atlases.
pub mod prelude {
    #[cfg(feature = "bevy")]
    /// Provides easy access to `Rpack` asset-related functionality in a Bevy application.
    pub use super::plugin::{
        RpackAssetHelper, RpackAssetPlugin, RpackAtlasAsset, RpackAtlasAssetError,
        RpackAtlasAssetLoader, RpackAtlasAssetLoaderSettings, RpackAtlasError, RpackAtlases,
    };
    /// Re-exports core types for working with texture atlases.
    pub use super::{AtlasAsset, AtlasFrame, SerializableRect};
}

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
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
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct AtlasFrame {
    /// A unique identifier for the frame.
    pub key: String,
    /// The rectangular area of the frame within the texture atlas.
    pub frame: SerializableRect,
}

/// Represents an entire texture atlas asset, including its metadata and frames.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "bevy", derive(Asset, Reflect))]
pub struct AtlasAsset {
    /// The overall dimensions of the texture atlas in pixels (width, height).
    pub size: [u32; 2],
    /// The filename associated with the atlas, typically used for loading or debugging.
    pub filename: String,
    /// A collection of frames contained within the texture atlas.
    pub frames: Vec<AtlasFrame>,
    /// Metadata about the atlas.
    #[cfg_attr(feature = "bevy", reflect(default))]
    #[serde(default, skip_serializing_if = "AtlasMetadata::skip_serialization")]
    pub metadata: AtlasMetadata,
}

/// Represents metadata associated with the texture atlas format.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct AtlasMetadata {
    /// The version of the texture atlas format.
    pub format_version: u32,
    /// The name of the application that created the atlas.
    pub app: Cow<'static, str>,
    /// The version of the application that created the atlas.
    pub app_version: Cow<'static, str>,
    /// Whether to skip serialization of the metadata.
    #[serde(skip_serializing, default)]
    pub skip_serialization: bool,
}

impl AtlasMetadata {
    /// Returns true if the metadata should be skipped during serialization.
    pub fn skip_serialization(&self) -> bool {
        self.skip_serialization
    }
}

impl Default for AtlasMetadata {
    fn default() -> Self {
        Self {
            format_version: 1,
            app: Cow::Borrowed("rpack"),
            app_version: Cow::Borrowed(env!("CARGO_PKG_VERSION")),
            skip_serialization: false,
        }
    }
}
