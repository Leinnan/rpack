use bevy_rpack::AtlasMetadata;
pub use bevy_rpack::packing;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
use std::io::Write;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use texture_packer::{TexturePackerConfig, importer::ImageImporter};
use thiserror::Error;

pub use crate::formats::SaveImageFormat;

pub mod formats;
#[deprecated(note = "Use packing::place for dimension-only atlas placement")]
pub mod packer;
pub mod saving;

#[derive(Clone)]
pub struct Spritesheet {
    pub image_data: DynamicImage,
    pub atlas_asset: bevy_rpack::AtlasAsset,
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
        let SpritesheetBuildConfig {
            packer_config: config,
            skip_metadata_serialization,
        } = config.into();
        let inputs: Vec<_> = images
            .iter()
            .map(|image| (image.id.as_str(), &image.image))
            .collect();
        let packed = packing::build(config, &inputs)?;
        let image_data = packed.pixels;
        let atlas_asset = bevy_rpack::AtlasAsset {
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

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TilemapGenerationConfig {
    pub asset_patterns: Vec<String>,
    pub output_path: String,
    /// Image format, png by default
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<SaveImageFormat>,
    /// Size of the tilemap texture. Default value is `2048`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<u32>,
    /// Size of the padding between frames in pixel. Default value is `2`
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub texture_padding: Option<u32>,
    /// Size of the padding on the outer edge of the packed image in pixel. Default value is `0`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub border_padding: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub skip_serializing_metadata: Option<bool>,
    #[serde(skip)]
    pub working_dir: Option<PathBuf>,
}

impl From<&TilemapGenerationConfig> for TexturePackerConfig {
    fn from(config: &TilemapGenerationConfig) -> Self {
        packing::packer_config(config.size, config.texture_padding, config.border_padding)
    }
}
impl From<&TilemapGenerationConfig> for SpritesheetBuildConfig {
    fn from(config: &TilemapGenerationConfig) -> Self {
        SpritesheetBuildConfig {
            packer_config: config.into(),
            skip_metadata_serialization: config.skip_serializing_metadata.unwrap_or_default(),
        }
    }
}

#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
impl TilemapGenerationConfig {
    pub fn read_from_file<P>(path: P) -> anyhow::Result<TilemapGenerationConfig>
    where
        P: AsRef<Path>,
    {
        let config_file = std::fs::read_to_string(path.as_ref())?;
        let mut config: TilemapGenerationConfig = serde_json::from_str(&config_file)?;
        config.working_dir = Path::new(path.as_ref()).parent().map(|p| p.to_path_buf());
        Ok(config)
    }

    /// Discover sorted, unique source files and propagate invalid patterns/read errors.
    pub fn try_file_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let working_dir = self.working_dir();
        let mut paths = std::collections::BTreeSet::new();
        for pattern in &self.asset_patterns {
            let full = working_dir.join(pattern);
            for path in glob::glob(&full.to_string_lossy())? {
                let path = path?;
                if path.is_file() {
                    paths.insert(path);
                }
            }
        }
        if paths.is_empty() {
            anyhow::bail!("no source images matched");
        }
        Ok(paths.into_iter().collect())
    }

    pub fn get_file_paths_and_prefix(&self) -> (Vec<PathBuf>, String) {
        let paths = self.try_file_paths().unwrap_or_default();
        let prefix = get_common_prefix(&paths);
        (paths, prefix)
    }

    pub fn working_dir(&self) -> PathBuf {
        let dir = match &self.working_dir {
            None => std::env::current_dir().expect("msg"),
            Some(p) => {
                if p.as_os_str().is_empty() {
                    std::env::current_dir().expect("msg")
                } else {
                    p.clone()
                }
            }
        };

        std::path::absolute(dir).unwrap_or_default()
    }

    pub fn generate(&self) -> anyhow::Result<()> {
        use crate::saving::SaveableImage;

        let working_dir = self.working_dir();

        packing::generation_config(self.size, self.texture_padding, self.border_padding)?;
        let file_paths = self.try_file_paths()?;
        let keys = packing::frame_keys(&file_paths)?;
        let images: Vec<ImageFile> = file_paths
            .iter()
            .zip(keys)
            .map(|(path, id)| {
                image::open(path)
                    .map(|image| ImageFile { id, image })
                    .map_err(|error| anyhow::anyhow!("could not load {}: {error}", path.display()))
            })
            .collect::<anyhow::Result<_>>()?;
        let atlas_image_path = working_dir.join(format!(
            "{}{}",
            self.output_path,
            self.format.unwrap_or_default()
        ));
        let atlas_filename = Path::new(&atlas_image_path)
            .file_name()
            .expect("D")
            .to_string_lossy()
            .to_string();
        let atlas_config_path = working_dir.join(format!("{}.rpack.json", self.output_path));
        let spritesheet = Spritesheet::build(self, &images, &atlas_filename)?;

        if Path::new(&atlas_config_path).exists() {
            std::fs::remove_file(&atlas_config_path).expect("Could not remove the old file");
        }
        if Path::new(&atlas_image_path).exists() {
            std::fs::remove_file(&atlas_image_path).expect("Could not remove the old file");
        }
        spritesheet
            .image_data
            .save_with_format_autodetection(&atlas_image_path)?;
        let json = serde_json::to_string_pretty(&spritesheet.atlas_asset_json)?;
        let mut file = std::fs::File::create(&atlas_config_path)?;
        file.write_all(json.as_bytes())?;
        println!(
            "Atlas from {} images saved at: {}",
            spritesheet.atlas_asset.frames.len(),
            atlas_config_path.display()
        );

        Ok(())
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
    #[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
    #[test]
    fn discovery_deduplicates_and_reports_errors() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy_rpack/assets/tilemap.png");
        let mut config = TilemapGenerationConfig {
            asset_patterns: vec![path.display().to_string(); 2],
            ..Default::default()
        };
        assert_eq!(config.try_file_paths().unwrap().len(), 1);
        config.asset_patterns = vec!["[".into()];
        assert!(config.try_file_paths().is_err());
        config.asset_patterns = vec!["missing-file-123.png".into()];
        assert!(config.try_file_paths().is_err());
    }
}
