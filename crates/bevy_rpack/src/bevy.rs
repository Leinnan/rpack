use crate::{AtlasAsset, SerializableRect};
use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::image::ImageSampler;
use bevy::{prelude::*, utils::HashMap};
use thiserror::Error;

/// This is an asset containing the texture atlas image, the texture atlas layout, and a map of the original file names to their corresponding indices in the texture atlas.
#[derive(Asset, Debug, Reflect)]
pub struct RpackAtlasAsset {
    /// The texture atlas image.
    pub image: Handle<Image>,
    /// The texture atlas layout.
    pub atlas: Handle<TextureAtlasLayout>,
    /// The map of the original file names to indices of the texture atlas.
    pub files: HashMap<String, usize>,
}

impl From<SerializableRect> for URect {
    fn from(val: SerializableRect) -> Self {
        URect {
            min: UVec2 { x: val.x, y: val.y },
            max: UVec2 {
                x: val.x + val.w,
                y: val.y + val.h,
            },
        }
    }
}

impl RpackAtlasAsset {
    // When atlas contains the given key returns a copy of TextureAtlas and Image
    pub fn get_atlas_data<T: AsRef<str>>(&self, key: T) -> Option<(TextureAtlas, Handle<Image>)> {
        self.files.get(key.as_ref()).map(|s| {
            (
                TextureAtlas {
                    index: *s,
                    layout: self.atlas.clone(),
                },
                self.image.clone(),
            )
        })
    }
    // When atlas contains the given key creates a Sprite component
    pub fn make_sprite<T: AsRef<str>>(&self, key: T) -> Option<Sprite> {
        if let Some((atlas, image)) = self.get_atlas_data(key) {
            Some(Sprite {
                image,
                texture_atlas: Some(atlas),
                ..Default::default()
            })
        } else {
            None
        }
    }
}

pub struct RpackAssetPlugin;

impl Plugin for RpackAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RpackAtlasAsset>();
        app.init_asset::<RpackAtlasAsset>();
        app.init_asset_loader::<RpackAtlasAssetLoader>();
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum RpackAtlasAssetError {
    /// An [IO](std::io) Error that occured
    /// during parsing of a `.rpack.json` file.
    #[error("could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse asset: {0}")]
    ParsinError(#[from] serde_json::Error),
    /// A Bevy [`LoadDirectError`](bevy::asset::LoadDirectError) that occured
    /// while loading a [`RpackAtlasAsset::image`](crate::RpackAtlasAsset::image).
    #[error("could not load asset: {0}")]
    LoadDirect(Box<bevy::asset::LoadDirectError>),
    /// An error that can occur if there is
    /// trouble loading the image asset of
    /// an atlas.
    #[error("missing image asset: {0}")]
    LoadingImageAsset(String),
}

impl From<bevy::asset::LoadDirectError> for RpackAtlasAssetError {
    fn from(value: bevy::asset::LoadDirectError) -> Self {
        Self::LoadDirect(Box::new(value))
    }
}

#[derive(Default)]
pub struct RpackAtlasAssetLoader;

impl AssetLoader for RpackAtlasAssetLoader {
    type Asset = RpackAtlasAsset;
    type Settings = ();
    type Error = RpackAtlasAssetError;

    fn extensions(&self) -> &[&str] {
        &["rpack.json"]
    }

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &(),
        load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut file = String::new();
        reader.read_to_string(&mut file).await?;
        let asset: AtlasAsset = serde_json::from_str(&file)?;

        let path = load_context
            .asset_path()
            .path()
            .parent()
            .unwrap_or(&std::path::Path::new(""))
            .join(asset.name);

        let mut image: Image = load_context
            .loader()
            .immediate()
            .with_unknown_type()
            .load(path)
            .await?
            .take()
            .ok_or(RpackAtlasAssetError::LoadingImageAsset(
                "failed to load image asset, does it exist".to_string(),
            ))?;
        image.sampler = ImageSampler::nearest();

        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(asset.size[0], asset.size[1]));
        let mut files = HashMap::new();

        for frame in asset.frames {
            let id = layout.add_texture(frame.frame.into());
            files.insert(frame.key, id);
        }

        let atlas = load_context.add_labeled_asset("atlas_layout".into(), layout);
        let image = load_context.add_labeled_asset("atlas_texture".into(), image);

        Ok(RpackAtlasAsset {
            image,
            atlas,
            files,
        })
    }
}
