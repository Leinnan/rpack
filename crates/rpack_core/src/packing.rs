//! Bevy-independent atlas placement, composition, and source naming.
use crate::{AtlasFrame, SerializableRect};
use image::{DynamicImage, GenericImage};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use texture_packer::{TexturePacker, TexturePackerConfig, texture::Texture};

// No pixels are accessed: placement disables trimming and never exports this texture.
#[derive(Clone)]
struct Dimensions(u32, u32);
impl Texture for Dimensions {
    type Pixel = image::Rgba<u8>;
    fn width(&self) -> u32 {
        self.0
    }
    fn height(&self) -> u32 {
        self.1
    }
    fn get(&self, _: u32, _: u32) -> Option<Self::Pixel> {
        None
    }
    fn set(&mut self, _: u32, _: u32, _: Self::Pixel) {
        unreachable!("dimension-only texture")
    }
}

/// A failure while preparing or packing an atlas.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PackingError {
    /// Invalid dimensions or an empty source list.
    #[error("invalid atlas config: {0}")]
    Config(String),
    /// Frame identifiers must be unique.
    #[error("duplicate atlas frame key: {0}")]
    DuplicateKey(String),
    /// A source does not fit.
    #[error("image does not fit in atlas: {0}")]
    Packing(String),
    /// Pixel export failed.
    #[error("could not export atlas: {0}")]
    Export(String),
}

/// Packed pixels and key-sorted frame metadata.
pub struct PackedAtlas {
    /// RGBA pixels for normal generation, upstream export for advanced options.
    pub pixels: DynamicImage,
    /// Frames sorted by key, independent of placement order.
    pub frames: Vec<AtlasFrame>,
}

/// Resolve generation defaults without applying the generation allocation limit.
pub fn packer_config(
    size: Option<u32>,
    padding: Option<u32>,
    border: Option<u32>,
) -> TexturePackerConfig {
    TexturePackerConfig {
        max_width: size.unwrap_or(2048),
        max_height: size.unwrap_or(2048),
        texture_padding: padding.unwrap_or(2),
        border_padding: border.unwrap_or(0),
        allow_rotation: false,
        force_max_dimensions: true,
        texture_extrusion: 0,
        trim: false,
        texture_outlines: false,
    }
}

/// Resolve and validate a generation configuration (maximum side: 8192 pixels).
pub fn generation_config(
    size: Option<u32>,
    padding: Option<u32>,
    border: Option<u32>,
) -> Result<TexturePackerConfig, PackingError> {
    let config = packer_config(size, padding, border);
    validate(config)?;
    if config.max_width > 8192 {
        return Err(PackingError::Config("size must be 1..=8192".into()));
    }
    Ok(config)
}

fn validate(config: TexturePackerConfig) -> Result<(), PackingError> {
    let border = config.border_padding.checked_mul(2);
    if config.max_width == 0
        || config.max_height == 0
        || border.is_none_or(|b| b >= config.max_width || b >= config.max_height)
        || config.texture_padding >= config.max_width
        || config.texture_padding >= config.max_height
    {
        return Err(PackingError::Config("dimensions must be positive, twice border_padding and texture_padding must be less than each dimension".into()));
    }
    config
        .texture_extrusion
        .checked_mul(2)
        .and_then(|e| e.checked_add(config.texture_padding))
        .and_then(|p| p.checked_add(config.max_width.max(config.max_height)))
        .ok_or_else(|| PackingError::Config("packing dimensions overflow".into()))?;
    Ok(())
}

fn normal(config: TexturePackerConfig) -> bool {
    !config.allow_rotation
        && !config.trim
        && config.texture_extrusion == 0
        && !config.texture_outlines
        && config.force_max_dimensions
}

/// Place dimensions in input order using the same rules as normal image generation.
/// Advanced exporter options require actual pixels and are rejected here.
pub fn place(
    config: TexturePackerConfig,
    sources: &[(&str, u32, u32)],
) -> Result<Vec<AtlasFrame>, PackingError> {
    validate(config)?;
    if !normal(config) {
        return Err(PackingError::Config(
            "dimension-only placement requires normal generation options".into(),
        ));
    }
    if sources.is_empty() {
        return Err(PackingError::Config("no source images matched".into()));
    }
    let width = config.max_width - 2 * config.border_padding;
    let height = config.max_height - 2 * config.border_padding;
    let overflow = || PackingError::Config("packing dimensions overflow".into());
    let mut packer = TexturePacker::new_skyline(TexturePackerConfig {
        max_width: width
            .checked_add(config.texture_padding)
            .ok_or_else(overflow)?,
        max_height: height
            .checked_add(config.texture_padding)
            .and_then(|h| h.checked_add(1))
            .ok_or_else(overflow)?,
        border_padding: 0,
        ..config
    });
    let mut seen = BTreeSet::new();
    let mut frames = Vec::with_capacity(sources.len());
    for &(key, w, h) in sources {
        if !seen.insert(key) {
            return Err(PackingError::DuplicateKey(key.into()));
        }
        let fail = || PackingError::Packing(key.into());
        if w == 0 || h == 0 || w > width || h > height {
            return Err(fail());
        }
        packer.pack_own(key, Dimensions(w, h)).map_err(|_| fail())?;
        let r = packer.get_frame(&key).ok_or_else(fail)?.frame;
        if r.x + r.w > width || r.y + r.h > height {
            return Err(fail());
        }
        frames.push(AtlasFrame {
            key: key.into(),
            frame: SerializableRect {
                x: r.x + config.border_padding,
                y: r.y + config.border_padding,
                w: r.w,
                h: r.h,
            },
        });
    }
    Ok(frames)
}

/// Build an atlas from borrowed images, preserving input placement order.
pub fn build(
    config: TexturePackerConfig,
    images: &[(&str, &DynamicImage)],
) -> Result<PackedAtlas, PackingError> {
    validate(config)?;
    if images.is_empty() {
        return Err(PackingError::Config("no source images matched".into()));
    }
    let mut seen = BTreeSet::new();
    for &(key, _) in images {
        if !seen.insert(key) {
            return Err(PackingError::DuplicateKey(key.into()));
        }
    }
    let (pixels, mut frames) = if normal(config) {
        let dimensions: Vec<_> = images
            .iter()
            .map(|(key, image)| (*key, image.width(), image.height()))
            .collect();
        let frames = place(config, &dimensions)?;
        let mut pixels = DynamicImage::new_rgba8(config.max_width, config.max_height);
        for ((_, image), frame) in images.iter().zip(&frames) {
            pixels
                .copy_from(*image, frame.frame.x, frame.frame.y)
                .map_err(|e| PackingError::Export(e.to_string()))?;
        }
        (pixels, frames)
    } else {
        let mut packer = TexturePacker::new_skyline(config);
        for &(key, image) in images {
            if image.width() == 0
                || image.height() == 0
                || image
                    .width()
                    .checked_add(config.texture_padding + config.texture_extrusion * 2)
                    .is_none()
                || image
                    .height()
                    .checked_add(config.texture_padding + config.texture_extrusion * 2)
                    .is_none()
                || !packer.can_pack(image)
            {
                return Err(PackingError::Packing(key.into()));
            }
            packer
                .pack_ref(key, image)
                .map_err(|_| PackingError::Packing(key.into()))?;
        }
        let pixels = texture_packer::exporter::ImageExporter::export(&packer, None)
            .map_err(|e| PackingError::Export(format!("{e:?}")))?;
        let frames = packer
            .get_frames()
            .values()
            .map(|f| AtlasFrame {
                key: f.key.into(),
                frame: SerializableRect {
                    x: f.frame.x,
                    y: f.frame.y,
                    w: f.frame.w,
                    h: f.frame.h,
                },
            })
            .collect();
        (pixels, frames)
    };
    frames.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(PackedAtlas { pixels, frames })
}

/// Normalize separators and remove only the final filename extension.
pub fn frame_key(path: &str) -> String {
    Path::new(&path.replace('\\', "/"))
        .with_extension("")
        .to_string_lossy()
        .into_owned()
}

/// Find a common directory by path components, never by partial directory names.
pub fn common_directory(paths: &[PathBuf]) -> PathBuf {
    let Some(first) = paths.first() else {
        return PathBuf::new();
    };
    let mut common = first.parent().unwrap_or(Path::new("")).to_path_buf();
    for path in paths {
        while !path.starts_with(&common) {
            if !common.pop() {
                return PathBuf::new();
            }
        }
    }
    common
}

/// Derive unique relative keys from normalized source paths.
pub fn frame_keys(paths: &[PathBuf]) -> Result<Vec<String>, PackingError> {
    let paths: Vec<_> = paths
        .iter()
        .map(|p| PathBuf::from(p.to_string_lossy().replace('\\', "/")))
        .collect();
    let common = common_directory(&paths);
    let mut seen = BTreeSet::new();
    paths
        .iter()
        .map(|path| {
            let key = frame_key(&path.strip_prefix(&common).unwrap_or(path).to_string_lossy());
            if !seen.insert(key.clone()) {
                return Err(PackingError::DuplicateKey(key));
            }
            Ok(key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba};
    fn solid(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(w, h, Rgba([255, 12, 0, 255])))
    }
    #[test]
    fn rectangular_exact_fit_and_transparent_border() {
        let config = TexturePackerConfig {
            max_height: 6,
            ..generation_config(Some(8), Some(2), Some(1)).unwrap()
        };
        let image = solid(6, 4);
        let packed = build(config, &[("a", &image)]).unwrap();
        let predicted = place(config, &[("a", 6, 4)]).unwrap();
        assert_eq!(
            serde_json::to_value(&packed.frames).unwrap(),
            serde_json::to_value(predicted).unwrap()
        );
        assert_eq!(packed.pixels.get_pixel(1, 1), Rgba([255, 12, 0, 255]));
        assert_eq!(packed.pixels.get_pixel(7, 5), Rgba([0; 4]));
        assert!(build(config, &[("a", &solid(7, 4))]).is_err());
    }
    #[test]
    fn validation_and_keys() {
        for size in [0, 8193, u32::MAX] {
            assert!(generation_config(Some(size), None, None).is_err());
        }
        assert!(generation_config(Some(8), Some(8), None).is_err());
        assert!(generation_config(Some(8), None, Some(u32::MAX)).is_err());
        let config = generation_config(Some(8), None, None).unwrap();
        assert!(build(config, &[]).is_err());
        assert!(place(config, &[("a", 0, 1)]).is_err());
        assert!(
            place(
                TexturePackerConfig {
                    max_height: u32::MAX,
                    ..config
                },
                &[("a", 1, 1)]
            )
            .is_err()
        );
        let image = solid(1, 1);
        assert!(matches!(
            build(config, &[("a", &image), ("a", &image)]),
            Err(PackingError::DuplicateKey(_))
        ));
        assert_eq!(
            frame_keys(&["tiles.v1/a/foo.bar.png".into(), "tiles.v1/b/x.png".into()]).unwrap(),
            ["a/foo.bar", "b/x"]
        );
        assert_eq!(frame_key("dir.v1\\image.test.png"), "dir.v1/image.test");
        assert!(frame_keys(&["a.png".into(), "a.jpg".into()]).is_err());
        assert!(frame_keys(&[]).unwrap().is_empty());
    }
    #[test]
    fn advanced_export_matches_upstream() {
        let image = solid(3, 2);
        for config in [
            TexturePackerConfig {
                allow_rotation: true,
                ..packer_config(Some(16), None, None)
            },
            TexturePackerConfig {
                trim: true,
                ..packer_config(Some(16), None, None)
            },
            TexturePackerConfig {
                texture_extrusion: 1,
                ..packer_config(Some(16), None, None)
            },
            TexturePackerConfig {
                texture_outlines: true,
                ..packer_config(Some(16), None, None)
            },
            TexturePackerConfig {
                force_max_dimensions: false,
                ..packer_config(Some(16), None, None)
            },
        ] {
            let mut upstream = TexturePacker::new_skyline(config);
            upstream.pack_ref("a", &image).unwrap();
            let expected =
                texture_packer::exporter::ImageExporter::export(&upstream, None).unwrap();
            assert_eq!(build(config, &[("a", &image)]).unwrap().pixels, expected);
        }
    }
}
