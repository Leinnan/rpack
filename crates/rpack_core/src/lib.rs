#![doc = include_str!("../README.md")]

mod atlas;
pub mod config;
pub mod formats;
pub mod packing;
pub mod pipeline;
pub mod sources;
pub mod spritesheet;

pub use atlas::{AtlasAsset, AtlasFrame, AtlasMetadata, SerializableRect};
pub use config::TilemapGenerationConfig;
pub use formats::SaveImageFormat;
pub use spritesheet::{
    ImageFile, Spritesheet, SpritesheetBuildConfig, SpritesheetError, get_common_prefix,
};
