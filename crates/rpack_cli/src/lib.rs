use bevy_rpack::{AtlasFrame, AtlasMetadata, SerializableRect};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
use std::io::Write;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use texture_packer::{TexturePacker, TexturePackerConfig, importer::ImageImporter};
use thiserror::Error;

pub use crate::formats::SaveImageFormat;

pub mod formats;
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
        let mut id = id.as_ref().to_owned().replace("\\", "/");
        if let Some((before, _)) = id.split_once('.') {
            id = before.to_string();
        }
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
    if paths.is_empty() {
        return String::new();
    }
    let path = Path::new(paths[0].as_ref())
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    let mut prefix = String::from(paths[0].as_ref().to_string_lossy())
        .strip_suffix(&path)
        .unwrap_or_default()
        .to_owned();

    for s in paths.iter().skip(1) {
        let s = s.as_ref().to_string_lossy();
        while !(s.starts_with(&prefix) || prefix.is_empty()) {
            prefix.pop();
        }
    }

    // Ensure the prefix ends at a directory boundary
    if !prefix.is_empty() && !prefix.ends_with('/') && !prefix.ends_with('\\') {
        if let Some(last_slash) = prefix.rfind('/') {
            prefix.truncate(last_slash + 1);
        } else if let Some(last_backslash) = prefix.rfind('\\') {
            prefix.truncate(last_backslash + 1);
        }
    }

    prefix
}

/// Errors that can occur while building a `Spritesheet`.
#[non_exhaustive]
#[derive(Debug, Error, Clone)]
pub enum SpritesheetError {
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
        let mut packer = TexturePacker::new_skyline(config);
        for image in images.iter() {
            if !packer.can_pack(&image.image) {
                return Err(SpritesheetError::CannotPackImage(image.id.clone()));
            }
            if let Err(_err) = packer.pack_ref(&image.id, &image.image) {
                return Err(SpritesheetError::FailedToPackImage);
            }
        }
        let Ok(image_data) = texture_packer::exporter::ImageExporter::export(&packer, None) else {
            return Err(SpritesheetError::FailedToExportImage);
        };

        let mut atlas_asset = bevy_rpack::AtlasAsset {
            metadata: AtlasMetadata {
                skip_serialization: skip_metadata_serialization,
                ..Default::default()
            },
            size: [image_data.width(), image_data.height()],
            filename: filename.as_ref().to_owned(),
            frames: packer
                .get_frames()
                .values()
                .map(|v| -> AtlasFrame {
                    AtlasFrame {
                        key: v.key.clone(),
                        frame: SerializableRect {
                            x: v.frame.x,
                            y: v.frame.y,
                            w: v.frame.w,
                            h: v.frame.h,
                        },
                    }
                })
                .collect(),
        };
        atlas_asset.frames.sort_by(|a, b| a.key.cmp(&b.key));
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
        texture_packer::TexturePackerConfig {
            max_width: config.size.unwrap_or(2048),
            max_height: config.size.unwrap_or(2048),
            allow_rotation: false,
            force_max_dimensions: true,
            border_padding: config.border_padding.unwrap_or(0),
            texture_padding: config.texture_padding.unwrap_or(2),
            texture_extrusion: 0,
            trim: false,
            texture_outlines: false,
        }
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

    pub fn get_file_paths_and_prefix(&self) -> (Vec<PathBuf>, String) {
        let working_dir = self.working_dir();
        let lossy_working_dir = working_dir.to_string_lossy();
        let mut file_paths: Vec<PathBuf> = self
            .asset_patterns
            .iter()
            .flat_map(|pattern| {
                let p = format!("{}/{}", lossy_working_dir, pattern);
                glob::glob(&p).expect("Wrong pattern for assets").flatten()
            })
            .filter(|e| e.is_file())
            .collect();
        file_paths.sort();
        let prefix = get_common_prefix(&file_paths);
        (file_paths, prefix)
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

        let (file_paths, prefix) = self.get_file_paths_and_prefix();
        let images: Vec<ImageFile> = file_paths
            .iter()
            .flat_map(|f| {
                let id = f
                    .to_str()
                    .unwrap_or_default()
                    .strip_prefix(&prefix)
                    .unwrap_or_default();
                ImageFile::at_path(f, id)
            })
            .collect();
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
