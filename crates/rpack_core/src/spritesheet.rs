//! Builds a finished atlas image and its JSON description.
use crate::{AtlasAsset, AtlasMetadata, packing};
use image::DynamicImage;
use serde_json::Value;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use texture_packer::{TexturePackerConfig, importer::ImageImporter};
use thiserror::Error;

#[derive(Clone)]
pub struct Spritesheet {
    pub image_data: DynamicImage,
    pub atlas_asset: AtlasAsset,
    pub atlas_asset_json: Value,
}

impl Spritesheet {
    pub fn rebuild_json(&mut self) {
        if let Ok(value) = serde_json::to_value(&self.atlas_asset) {
            self.atlas_asset_json = value;
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ImageFile {
    pub id: String,
    pub image: DynamicImage,
}

impl ImageFile {
    pub fn at_path<P>(path: &Path, id: P) -> Option<ImageFile>
    where
        P: AsRef<str>,
    {
        let id = packing::frame_key(id.as_ref());
        if let Ok(image) = ImageImporter::import_from_file(path) {
            Some(ImageFile { image, id })
        } else {
            None
        }
    }
}

pub fn get_common_prefix<S>(paths: &[S]) -> String
where
    S: AsRef<OsStr> + Sized,
{
    let paths: Vec<_> = paths
        .iter()
        .map(|p| PathBuf::from(p.as_ref().to_string_lossy().replace('\\', "/")))
        .collect();
    let mut prefix = packing::common_directory(&paths)
        .to_string_lossy()
        .into_owned();
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    prefix
}

/// Errors that can occur while building a `Spritesheet`.
#[non_exhaustive]
#[derive(Debug, Error, Clone)]
pub enum SpritesheetError {
    #[error(transparent)]
    Packing(#[from] packing::PackingError),
    #[error("Cannot pack image: {0}")]
    CannotPackImage(String),
    #[error("Failed to export tilemap image")]
    FailedToExportImage,
    #[error("could not parse asset: {0}")]
    ParsingError(String),
    #[error("Failed to pack image into tilemap, tilemap to small")]
    FailedToPackImage,
}

/// Configuration for building a `Spritesheet`.
#[derive(Debug, Clone)]
pub struct SpritesheetBuildConfig {
    /// Configuration for the texture packer.
    pub packer_config: TexturePackerConfig,
    /// Whether to skip metadata serialization.
    pub skip_metadata_serialization: bool,
}

impl From<TexturePackerConfig> for SpritesheetBuildConfig {
    fn from(config: TexturePackerConfig) -> Self {
        Self {
            packer_config: config,
            skip_metadata_serialization: false,
        }
    }
}

impl Spritesheet {
    pub fn build<P>(
        config: impl Into<SpritesheetBuildConfig>,
        images: &[ImageFile],
        filename: P,
    ) -> Result<Self, SpritesheetError>
    where
        P: AsRef<str>,
    {
        let inputs: Vec<_> = images
            .iter()
            .map(|image| (image.id.as_str(), &image.image))
            .collect();
        Self::from_images(config, &inputs, filename)
    }

    /// Same as [`Spritesheet::build`], for callers that keep ownership of the images.
    pub fn from_images<P>(
        config: impl Into<SpritesheetBuildConfig>,
        images: &[(&str, &DynamicImage)],
        filename: P,
    ) -> Result<Self, SpritesheetError>
    where
        P: AsRef<str>,
    {
        let SpritesheetBuildConfig {
            packer_config: config,
            skip_metadata_serialization,
        } = config.into();
        let packed = packing::build(config, images)?;
        let image_data = packed.pixels;
        let atlas_asset = AtlasAsset {
            metadata: AtlasMetadata {
                skip_serialization: skip_metadata_serialization,
                ..Default::default()
            },
            size: [image_data.width(), image_data.height()],
            filename: filename.as_ref().to_owned(),
            frames: packed.frames,
        };
        let atlas_asset_json = serde_json::to_value(&atlas_asset)
            .map_err(|e| SpritesheetError::ParsingError(e.to_string()))?;

        Ok(Spritesheet {
            image_data,
            atlas_asset,
            atlas_asset_json,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapper_preserves_pixels_frames_and_metadata() {
        let images = vec![
            ImageFile {
                id: "b".into(),
                image: DynamicImage::new_rgba8(2, 2),
            },
            ImageFile {
                id: "a".into(),
                image: DynamicImage::new_rgba8(2, 2),
            },
        ];
        let config = packing::generation_config(Some(8), Some(2), Some(1)).unwrap();
        let core = packing::build(
            config,
            &images
                .iter()
                .map(|i| (i.id.as_str(), &i.image))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        for skip in [false, true] {
            let sheet = Spritesheet::build(
                SpritesheetBuildConfig {
                    packer_config: config,
                    skip_metadata_serialization: skip,
                },
                &images,
                "atlas.png",
            )
            .unwrap();
            assert_eq!(sheet.image_data, core.pixels);
            assert_eq!(
                serde_json::to_value(sheet.atlas_asset.frames).unwrap(),
                serde_json::to_value(&core.frames).unwrap()
            );
            assert_eq!(sheet.atlas_asset_json.get("metadata").is_none(), skip);
        }
    }
}
