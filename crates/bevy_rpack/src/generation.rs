//! Runtime generation of atlases from asset-source images.
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use bevy_asset::{AssetLoader, AssetPath, AssetServer, AssetServerMode, AsyncReadExt, LoadContext};
use bevy_ecs::world::{FromWorld, World};
use bevy_image::{Image, TextureAtlasLayout};
#[cfg(test)]
use bevy_math::URect;
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use futures_lite::StreamExt;
use image::DynamicImage;
use texture_packer::TexturePackerConfig;
use thiserror::Error;

use crate::plugin::{RpackAtlasAsset, RpackAtlasAssetLoaderSettings};

/// Configuration compatible with the generation fields of CLI `.rpack_gen.json` files.
/// Paths are relative to this asset, within the same asset source. Export-only CLI
/// fields are accepted but ignored: generation never writes files.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RpackAtlasGenConfig {
    /// Explicit image paths or glob patterns. Directory enumeration is needed for globs.
    pub asset_patterns: Vec<String>,
    /// Square atlas size in pixels; defaults to 2048.
    pub size: Option<u32>,
    /// Space between frames; defaults to 2 pixels.
    pub texture_padding: Option<u32>,
    /// Space around the atlas; defaults to 0 pixels.
    pub border_padding: Option<u32>,
}

/// Errors encountered while generating an atlas.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RpackAtlasGenError {
    /// Reading the configuration failed.
    #[error("could not read atlas config: {0}")]
    Io(#[from] std::io::Error),
    /// The configuration was not valid JSON.
    #[error("could not parse atlas config: {0}")]
    Json(#[from] serde_json::Error),
    /// Invalid dimensions, paths, or patterns.
    #[error("invalid atlas config: {0}")]
    Config(String),
    /// Source access, enumeration, or image decoding failed.
    #[error("could not load atlas source {path}: {message}")]
    Source {
        /// Asset path involved in the failure.
        path: String,
        /// Underlying failure.
        message: String,
    },
    /// Two source paths produced the same frame key.
    #[error("duplicate atlas frame key: {0}")]
    DuplicateKey(String),
    /// A source image could not fit in the configured atlas.
    #[error("image does not fit in atlas: {0}")]
    Packing(String),
    /// Exporting packed pixels failed.
    #[error("could not export atlas: {0}")]
    Export(String),
}

/// Loads `.rpack_gen.json` into an in-memory [`RpackAtlasAsset`].
#[derive(TypePath)]
pub struct RpackAtlasGenConfigLoader {
    server: AssetServer,
}

impl FromWorld for RpackAtlasGenConfigLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            server: world.resource::<AssetServer>().clone(),
        }
    }
}

impl RpackAtlasGenConfig {
    fn packer_config(&self) -> Result<TexturePackerConfig, RpackAtlasGenError> {
        crate::packing::generation_config(self.size, self.texture_padding, self.border_padding)
            .map_err(Into::into)
    }
}

fn relative_pattern(value: &str) -> Result<PathBuf, RpackAtlasGenError> {
    let normalized = value.replace('\\', "/");
    let path = PathBuf::from(&normalized);
    if normalized.is_empty()
        || normalized.contains(':')
        || normalized.contains('#')
        || path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RpackAtlasGenError::Config(format!(
            "expected a relative path within the config directory: {value}"
        )));
    }
    Ok(path.components().collect())
}

impl From<crate::packing::PackingError> for RpackAtlasGenError {
    fn from(error: crate::packing::PackingError) -> Self {
        use crate::packing::PackingError;
        match error {
            PackingError::Config(message) => Self::Config(message),
            PackingError::DuplicateKey(key) => Self::DuplicateKey(key),
            PackingError::Packing(key) => Self::Packing(key),
            PackingError::Export(message) => Self::Export(message),
        }
    }
}

fn frame_keys(paths: &[PathBuf]) -> Result<Vec<String>, RpackAtlasGenError> {
    crate::packing::frame_keys(paths).map_err(Into::into)
}

fn build_atlas(
    config: TexturePackerConfig,
    images: &[(String, DynamicImage)],
) -> Result<(DynamicImage, TextureAtlasLayout, HashMap<String, usize>), RpackAtlasGenError> {
    let images: Vec<_> = images
        .iter()
        .map(|(key, image)| (key.as_str(), image))
        .collect();
    let packed = crate::packing::build(config, &images)?;
    let (layout, files) = crate::plugin::atlas_layout(
        [packed.pixels.width(), packed.pixels.height()],
        packed.frames,
    );
    Ok((packed.pixels, layout, files))
}

impl AssetLoader for RpackAtlasGenConfigLoader {
    type Asset = RpackAtlasAsset;
    type Settings = RpackAtlasAssetLoaderSettings;
    type Error = RpackAtlasGenError;

    fn extensions(&self) -> &[&str] {
        &["rpack_gen.json"]
    }

    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        settings: &Self::Settings,
        context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut json = String::new();
        reader.read_to_string(&mut json).await?;
        let config: RpackAtlasGenConfig = serde_json::from_str(&json)?;
        let packing = config.packer_config()?;
        let base = context
            .path()
            .path()
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let source_id = context.path().source().clone_owned();
        let source = self
            .server
            .get_source(source_id.clone())
            .map_err(|e| RpackAtlasGenError::Config(e.to_string()))?;
        let asset_reader = match self.server.mode() {
            AssetServerMode::Unprocessed => source.reader(),
            AssetServerMode::Processed => source
                .processed_reader()
                .map_err(|e| RpackAtlasGenError::Config(e.to_string()))?,
        };
        let mut paths = BTreeSet::new();
        for value in &config.asset_patterns {
            let relative = relative_pattern(value)?;
            let full = base.join(&relative);
            let pattern = glob::Pattern::new(&relative.to_string_lossy())
                .map_err(|e| RpackAtlasGenError::Config(format!("{value}: {e}")))?;
            if !value.contains(['*', '?', '[']) {
                paths.insert(full);
                continue;
            }
            let mut root = base.clone();
            for component in relative.components() {
                if component
                    .as_os_str()
                    .to_string_lossy()
                    .contains(['*', '?', '['])
                {
                    break;
                }
                root.push(component);
            }
            let mut pending = vec![root];
            while let Some(directory) = pending.pop() {
                let mut entries = asset_reader.read_directory(&directory).await.map_err(|e| {
                    RpackAtlasGenError::Source {
                        path: directory.display().to_string(),
                        message: format!(
                            "glob enumeration failed (use explicit image paths if unsupported): {e}"
                        ),
                    }
                })?;
                while let Some(path) = entries.next().await {
                    if asset_reader.is_directory(&path).await.map_err(|e| {
                        RpackAtlasGenError::Source {
                            path: path.display().to_string(),
                            message: e.to_string(),
                        }
                    })? {
                        pending.push(path);
                    } else if pattern.matches_path_with(
                        path.strip_prefix(&base)
                            .map_err(|e| RpackAtlasGenError::Source {
                                path: path.display().to_string(),
                                message: e.to_string(),
                            })?,
                        glob::MatchOptions {
                            require_literal_separator: true,
                            ..Default::default()
                        },
                    ) {
                        paths.insert(path);
                    }
                }
            }
        }
        if paths.is_empty() {
            return Err(RpackAtlasGenError::Config(
                "no source images matched".into(),
            ));
        }
        let paths: Vec<_> = paths.into_iter().collect();
        let keys = frame_keys(&paths)?;
        let mut images = Vec::new();
        for (path, key) in paths.into_iter().zip(keys) {
            let asset_path = AssetPath::from(path.clone()).with_source(source_id.clone());
            let bytes = context.read_asset_bytes(asset_path).await.map_err(|e| {
                RpackAtlasGenError::Source {
                    path: path.display().to_string(),
                    message: e.to_string(),
                }
            })?;
            let image =
                image::load_from_memory(&bytes).map_err(|e| RpackAtlasGenError::Source {
                    path: path.display().to_string(),
                    message: e.to_string(),
                })?;
            images.push((key, DynamicImage::ImageRgba8(image.into_rgba8())));
        }
        let (pixels, layout, files) = build_atlas(packing, &images)?;
        let mut image = Image::from_dynamic(pixels, true, Default::default());
        image.sampler = settings.image_sampler.clone();
        Ok(RpackAtlasAsset {
            image: context.add_labeled_asset("atlas_texture", image),
            atlas: context.add_labeled_asset("atlas_layout", layout),
            files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba};

    fn solid(w: u32, h: u32, color: [u8; 4]) -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(w, h, Rgba(color)))
    }

    #[test]
    fn config_and_keys() {
        let config: RpackAtlasGenConfig = serde_json::from_str(
            r#"{"asset_patterns":["tiles/*"],"output_path":"ignored","format":"Png"}"#,
        )
        .unwrap();
        assert_eq!(config.packer_config().unwrap().max_width, 2048);
        for size in [0, 8193, u32::MAX] {
            assert!(
                RpackAtlasGenConfig {
                    size: Some(size),
                    ..config.clone()
                }
                .packer_config()
                .is_err()
            );
        }
        assert!(relative_pattern("../outside.png").is_err());
        assert!(relative_pattern("/outside.png").is_err());
        assert_eq!(
            relative_pattern("tiles\\a.png").unwrap(),
            PathBuf::from("tiles/a.png")
        );
        assert_eq!(
            frame_keys(&["tiles/a/foo.bar.png".into(), "tiles/b/x.png".into()]).unwrap(),
            ["a/foo.bar", "b/x"]
        );
        assert!(matches!(
            frame_keys(&["a.png".into(), "a.jpg".into()]),
            Err(RpackAtlasGenError::DuplicateKey(_))
        ));
    }

    #[test]
    fn exact_fit_border_pixels_and_overflow() {
        let config = RpackAtlasGenConfig {
            size: Some(8),
            border_padding: Some(1),
            texture_padding: Some(2),
            ..Default::default()
        }
        .packer_config()
        .unwrap();
        let images = vec![("a".into(), solid(6, 6, [255, 0, 0, 255]))];
        let (pixels, layout, files) = build_atlas(config, &images).unwrap();
        assert_eq!(layout.textures[files["a"]], URect::new(1, 1, 7, 7));
        assert_eq!(pixels.get_pixel(0, 0), Rgba([0, 0, 0, 0]));
        assert_eq!(pixels.get_pixel(6, 6), Rgba([255, 0, 0, 255]));
        assert_eq!(pixels.get_pixel(7, 7), Rgba([0, 0, 0, 0]));
        assert!(build_atlas(config, &[("large".into(), solid(7, 6, [0; 4]))]).is_err());
    }

    #[test]
    fn deterministic_layout_and_padding() {
        let config = RpackAtlasGenConfig {
            size: Some(8),
            texture_padding: Some(2),
            ..Default::default()
        }
        .packer_config()
        .unwrap();
        let images = vec![
            ("b".into(), solid(2, 2, [0, 255, 0, 255])),
            ("a".into(), solid(2, 2, [255, 0, 0, 255])),
        ];
        let (pixels, layout, files) = build_atlas(config, &images).unwrap();
        let packed = crate::packing::build(
            config,
            &images
                .iter()
                .map(|(key, image)| (key.as_str(), image))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(pixels, packed.pixels);
        for (index, frame) in packed.frames.iter().enumerate() {
            assert_eq!(files[&frame.key], index);
            assert_eq!(layout.textures[index], frame.frame.into());
        }

        let (again, other, _) = build_atlas(config, &images).unwrap();
        assert_eq!(pixels, again);
        assert_eq!(layout.textures, other.textures);
        assert_eq!(files["a"], 0);
        assert_eq!(pixels.get_pixel(2, 0), Rgba([0; 4]));
        assert_eq!(pixels.get_pixel(4, 0), Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn loads_globs_labels_and_reloads_sources() {
        use bevy_app::{App, TaskPoolPlugin};
        use bevy_asset::{
            AssetApp, AssetPlugin, Assets,
            io::{
                AssetSourceBuilder, AssetSourceId,
                memory::{Dir, MemoryAssetReader},
            },
        };
        let dir = Dir::default();
        let png = |color| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            solid(2, 2, color)
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            bytes.into_inner()
        };
        dir.insert_asset_text(
            Path::new("atlas/test.rpack_gen.json"),
            r#"{"asset_patterns":["images/*.png","images/red.png"],"size":8}"#,
        );
        dir.insert_asset(Path::new("atlas/images/red.png"), png([255, 0, 0, 255]));
        let source_dir = dir.clone();
        struct Watcher;
        impl bevy_asset::io::AssetWatcher for Watcher {}
        let (watch_tx, watch_rx) = std::sync::mpsc::channel();
        let mut app = App::new();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: source_dir.clone(),
                })
            })
            .with_watcher(move |sender| {
                watch_tx.send(sender).unwrap();
                Some(Box::new(Watcher))
            }),
        );
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin {
                watch_for_changes_override: Some(true),
                ..Default::default()
            },
        ));
        let events = watch_rx.recv().unwrap();
        app.init_asset::<Image>().init_asset::<TextureAtlasLayout>();
        app.add_plugins(crate::plugin::RpackAssetPlugin);
        let server = app.world().resource::<AssetServer>().clone();
        let handle = server.load::<RpackAtlasAsset>("atlas/test.rpack_gen.json");
        let wait_for_color = |app: &mut App, color| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                app.update();
                if let Some(atlas) = app
                    .world()
                    .resource::<Assets<RpackAtlasAsset>>()
                    .get(&handle)
                {
                    assert_eq!(atlas.files.len(), 1);
                    assert!(
                        app.world()
                            .resource::<Assets<TextureAtlasLayout>>()
                            .get(&atlas.atlas)
                            .is_some()
                    );
                    if let Some(image) = app.world().resource::<Assets<Image>>().get(&atlas.image) {
                        if image.clone().try_into_dynamic().unwrap().get_pixel(0, 0) == Rgba(color)
                        {
                            break;
                        }
                    }
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "load failed or timed out: {:?}",
                    server.load_state(handle.id())
                );
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        wait_for_color(&mut app, [255, 0, 0, 255]);
        dir.insert_asset(Path::new("atlas/images/red.png"), png([0, 255, 0, 255]));
        events
            .try_send(bevy_asset::io::AssetSourceEvent::ModifiedAsset(
                "atlas/images/red.png".into(),
            ))
            .unwrap();
        wait_for_color(&mut app, [0, 255, 0, 255]);
    }
}
