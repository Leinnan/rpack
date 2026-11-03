//! Finds a fitting atlas size and builds the atlas, as the GUI does on every change.
use crate::{
    Spritesheet, SpritesheetBuildConfig, SpritesheetError, TilemapGenerationConfig, packing,
};
use image::DynamicImage;
use texture_packer::TexturePackerConfig;

/// Atlas sides offered by the size search, smallest first.
pub const ATLAS_SIZES: [u32; 8] = [32, 64, 128, 256, 512, 1024, 2048, 4096];

/// Result of [`build_atlas`].
pub struct AtlasBuild {
    /// Smallest square atlas side that fits all images, `0` if even the largest does not.
    pub min_size: u32,
    pub result: Result<Spritesheet, SpritesheetError>,
}

/// Finds the smallest square atlas side that fits the images.
/// Returns `0` and the last packing error when none of the supported sizes does.
pub fn find_min_size(
    config: TexturePackerConfig,
    dimensions: &[(&str, u32, u32)],
) -> (u32, Option<SpritesheetError>) {
    if dimensions.is_empty() {
        return (ATLAS_SIZES[0], None);
    }
    let mut last_error = None;
    for size in ATLAS_SIZES {
        match packing::place(
            TexturePackerConfig {
                max_width: size,
                max_height: size,
                ..config
            },
            dimensions,
        ) {
            Ok(_) => return (size, None),
            Err(error) => last_error = Some(error.into()),
        }
    }
    (0, last_error)
}

/// Builds an atlas named `filename`, growing past the configured size when the images need it.
pub fn build_atlas(
    settings: &TilemapGenerationConfig,
    images: &[(&str, &DynamicImage)],
    filename: &str,
) -> AtlasBuild {
    let config: TexturePackerConfig = settings.into();
    let dimensions: Vec<_> = images
        .iter()
        .map(|(id, image)| (*id, image.width(), image.height()))
        .collect();
    let (min_size, error) = find_min_size(config, &dimensions);
    if min_size == 0 {
        return AtlasBuild {
            min_size,
            result: Err(error.unwrap_or(SpritesheetError::FailedToPackImage)),
        };
    }
    let mut packer_config = config;
    packer_config.max_width = packer_config.max_width.max(min_size);
    packer_config.max_height = packer_config.max_height.max(min_size);
    let build_config = SpritesheetBuildConfig {
        packer_config,
        skip_metadata_serialization: settings.skip_serializing_metadata.unwrap_or_default(),
    };
    AtlasBuild {
        min_size,
        result: Spritesheet::from_images(build_config, images, filename),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn min_size(settings: &TilemapGenerationConfig, width: u32, height: u32) -> (u32, bool) {
        let (size, error) = find_min_size(settings.into(), &[("tile", width, height)]);
        (size, error.is_some())
    }

    #[test]
    fn minimum_size_matches_builder_and_reports_exhaustion() {
        let settings = TilemapGenerationConfig {
            border_padding: Some(1),
            ..Default::default()
        };
        assert_eq!(min_size(&settings, 30, 30), (32, false));
        let config = packing::generation_config(Some(32), None, Some(1)).unwrap();
        let image = DynamicImage::new_rgba8(30, 30);
        assert!(Spritesheet::from_images(config, &[("tile", &image)], "atlas.png").is_ok());
        assert_eq!(min_size(&settings, 4096, 1), (0, true));
    }

    #[test]
    fn build_grows_past_a_too_small_configured_size() {
        let settings = TilemapGenerationConfig {
            size: Some(32),
            texture_padding: Some(0),
            ..Default::default()
        };
        let image = DynamicImage::new_rgba8(100, 100);
        let build = build_atlas(&settings, &[("tile", &image)], "atlas.png");
        assert_eq!(build.min_size, 128);
        assert_eq!(build.result.unwrap().atlas_asset.filename, "atlas.png");
    }
}
