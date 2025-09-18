use bevy_rpack::{AtlasFrame, AtlasMetadata, SerializableRect};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
use std::io::Write;
use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};
use texture_packer::{TexturePacker, TexturePackerConfig, importer::ImageImporter};
use thiserror::Error;

pub mod packer;

#[derive(Clone)]
pub struct Spritesheet {
    pub image_data: DynamicImage,
    pub atlas_asset: bevy_rpack::AtlasAsset,
    pub atlas_asset_json: Value,
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

#[derive(Clone, Debug, Default, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    all(feature = "cli", not(target_arch = "wasm32")),
    derive(clap::ValueEnum)
)]
pub enum SaveImageFormat {
    #[default]
    Png,
    Dds,
    #[cfg(feature = "basis")]
    Basis,
}

impl Display for SaveImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveImageFormat::Png => f.write_str(".png"),
            SaveImageFormat::Dds => f.write_str(".dds"),
            #[cfg(feature = "basis")]
            SaveImageFormat::Basis => f.write_str(".basis"),
        }
    }
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

    #[cfg(all(feature = "dds", not(target_arch = "wasm32")))]
    pub fn save_as_dds<R>(&self, output_path: R)
    where
        R: AsRef<Path>,
    {
        let rgba_image = self.image_data.to_rgba8();

        let dds = image_dds::dds_from_image(
            &rgba_image,
            image_dds::ImageFormat::Rgba8Unorm,
            image_dds::Quality::Fast,
            image_dds::Mipmaps::GeneratedAutomatic,
        )
        .unwrap();

        let mut writer =
            std::io::BufWriter::new(std::fs::File::create(output_path.as_ref()).unwrap());
        dds.write(&mut writer).unwrap();
    }

    #[cfg(all(feature = "basis", not(target_arch = "wasm32")))]
    pub fn save_as_basis<R>(&self, output_path: R) -> anyhow::Result<()>
    where
        R: AsRef<Path>,
    {
        use basis_universal::{BasisTextureFormat, Compressor, Transcoder};
        use image::{EncodableLayout, GenericImageView};

        let rgba_image = self.image_data.to_rgba8();

        let channel_count = 4;
        let (pixel_width, pixel_height) = self.image_data.dimensions();
        let mut compressor_params = basis_universal::CompressorParams::new();
        compressor_params.set_generate_mipmaps(true);
        compressor_params.set_basis_format(BasisTextureFormat::ETC1S);
        compressor_params.set_etc1s_quality_level(basis_universal::ETC1S_QUALITY_MAX);
        compressor_params.set_print_status_to_stdout(false);
        let mut compressor_image = compressor_params.source_image_mut(0);
        compressor_image.init(
            rgba_image.as_bytes(),
            pixel_width,
            pixel_height,
            channel_count,
        );

        //
        // Create the compressor and compress
        //
        let mut compressor = Compressor::default();
        let compression_time = unsafe {
            compressor.init(&compressor_params);
            let t0 = std::time::Instant::now();
            compressor.process().expect("Failed to compress the image.");
            let t1 = std::time::Instant::now();
            t1 - t0
        };

        // You could write it to disk like this
        let basis_file = compressor.basis_file();
        let transcoder = Transcoder::new();
        let mip_level_count = transcoder.image_level_count(basis_file, 0);
        println!(
            "Compressed {} mip levels to {} total bytes in {} ms",
            mip_level_count,
            compressor.basis_file_size(),
            compression_time.as_secs_f64() * 1000.0
        );
        std::fs::write(output_path.as_ref(), basis_file)?;
        Ok(())
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
        let working_dir = std::path::absolute(dir).unwrap_or_default();

        working_dir
    }

    pub fn generate(&self) -> anyhow::Result<()> {
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
        match self.format.unwrap_or_default() {
            SaveImageFormat::Dds => {
                #[cfg(feature = "dds")]
                spritesheet.save_as_dds(&atlas_image_path);
                #[cfg(not(feature = "dds"))]
                panic!(
                    "Program is compiled without support for dds. Compile it yourself with feature `dds` enabled."
                );
            }
            SaveImageFormat::Png => {
                spritesheet
                    .image_data
                    .save_with_format(&atlas_image_path, image::ImageFormat::Png)?;
            }
            #[cfg(feature = "basis")]
            SaveImageFormat::Basis => {
                spritesheet.save_as_basis(&atlas_image_path)?;
                panic!(
                    "Program is compiled without support for basis. Compile it yourself with feature `basis` enabled."
                );
            }
        }
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
