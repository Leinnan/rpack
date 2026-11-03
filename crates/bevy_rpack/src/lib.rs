#![doc = include_str!("../README.md")]

#[cfg(feature = "bevy")]
/// Contains the Bevy plugin for handling `Rpack` assets and atlases.
mod plugin;

#[cfg(feature = "bevy")]
mod generation;

pub use rpack_core::{AtlasAsset, AtlasFrame, AtlasMetadata, SerializableRect, packing};

/// Re-exports all types for working with texture atlases.
pub mod prelude {
    #[cfg(feature = "bevy")]
    pub use super::generation::{
        RpackAtlasGenConfig, RpackAtlasGenConfigLoader, RpackAtlasGenError,
    };
    #[cfg(feature = "bevy")]
    /// Provides easy access to `Rpack` asset-related functionality in a Bevy application.
    pub use super::plugin::{
        RpackAssetHelper, RpackAssetPlugin, RpackAtlasAsset, RpackAtlasAssetError,
        RpackAtlasAssetLoader, RpackAtlasAssetLoaderSettings, RpackAtlasError, RpackAtlases,
    };
    /// Re-exports core types for working with texture atlases.
    pub use super::{AtlasAsset, AtlasFrame, SerializableRect};
}
