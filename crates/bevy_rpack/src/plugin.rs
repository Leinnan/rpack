use crate::{AtlasAsset, SerializableRect};
use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::ecs::system::SystemParam;
use bevy::image::ImageSampler;
use bevy::{prelude::*, utils::HashMap};
use thiserror::Error;

/// Errors that can occur while accessing and creating components from [`RpackAtlasAsset`].
#[derive(Debug, Error)]
pub enum RpackAtlasError {
    /// An error that occured due to no atlas being loaded yet
    #[error("There is no atlas.")]
    NoAtlas,
    /// An error that occured because atlas does not contain provided key.
    #[error("There is no frame with provided key.")]
    WrongKey,
}

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

/// SystemParam helper for accessing and creating components from `Rpack` atlas data.
#[derive(SystemParam, DerefMut, Deref)]
pub struct RpackAtlases<'w>(pub Res<'w, Assets<RpackAtlasAsset>>);

/// A helper trait for accessing and creating components from `Rpack` atlas data.
#[allow(dead_code)]
pub trait RpackAssetHelper {
    /// Retrieves the atlas data (texture atlas and image) for the given atlas key, if available in any of the loaded Atlases.
    fn find_atlas_data_by_key<T: AsRef<str>>(
        &self,
        key: T,
    ) -> Result<(TextureAtlas, Handle<Image>), RpackAtlasError>;
    /// Creates a [`Sprite`] component for the given atlas key, if available in any of the loaded Atlases.
    fn try_make_sprite<T: AsRef<str>>(&self, key: T) -> Result<Sprite, RpackAtlasError>;
    /// Creates a [`ImageNode`] component for the given atlas key, if available in any of the loaded Atlases.
    fn try_make_image_node<T: AsRef<str>>(&self, key: T) -> Result<ImageNode, RpackAtlasError>;

    /// Provides list of all loaded atlas data keys
    fn atlas_data_keys(&self) -> Vec<&str>;
}

impl RpackAssetHelper for Assets<RpackAtlasAsset> {
    fn atlas_data_keys(&self) -> Vec<&str> {
        self.iter()
            .flat_map(|(_, e)| e.files.keys().map(|e| e.as_ref()))
            .collect()
    }
    fn find_atlas_data_by_key<T: AsRef<str>>(
        &self,
        key: T,
    ) -> Result<(TextureAtlas, Handle<Image>), RpackAtlasError> {
        if self.is_empty() {
            return Err(RpackAtlasError::NoAtlas);
        }
        for (_, a) in self.iter() {
            if let Ok(atlas_data) = a.get_atlas_data(key.as_ref()) {
                return Ok(atlas_data);
            }
        }
        Err(RpackAtlasError::WrongKey)
    }

    fn try_make_sprite<T: AsRef<str>>(&self, key: T) -> Result<Sprite, RpackAtlasError> {
        if self.is_empty() {
            return Err(RpackAtlasError::NoAtlas);
        }
        for (_, a) in self.iter() {
            if let Ok(sprite) = a.try_make_sprite(key.as_ref()) {
                return Ok(sprite);
            }
        }
        Err(RpackAtlasError::WrongKey)
    }

    fn try_make_image_node<T: AsRef<str>>(&self, key: T) -> Result<ImageNode, RpackAtlasError> {
        if self.is_empty() {
            return Err(RpackAtlasError::NoAtlas);
        }
        for (_, a) in self.iter() {
            if let Ok(image_node) = a.try_make_image_node(key.as_ref()) {
                return Ok(image_node);
            }
        }
        Err(RpackAtlasError::WrongKey)
    }
}

impl RpackAtlasAsset {
    /// Retrieves the atlas data (texture atlas and image) for the given atlas key, if available.
    pub fn get_atlas_data<T: AsRef<str>>(
        &self,
        key: T,
    ) -> Result<(TextureAtlas, Handle<Image>), RpackAtlasError> {
        match self.files.get(key.as_ref()) {
            Some(s) => Ok((
                TextureAtlas {
                    index: *s,
                    layout: self.atlas.clone(),
                },
                self.image.clone(),
            )),
            _ => Err(RpackAtlasError::WrongKey),
        }
    }

    /// Creates a [`Sprite`] component for the given atlas key
    pub fn try_make_sprite<T: AsRef<str>>(&self, key: T) -> Result<Sprite, RpackAtlasError> {
        if let Ok((atlas, image)) = self.get_atlas_data(key) {
            Ok(Sprite::from_atlas_image(image, atlas))
        } else {
            Err(RpackAtlasError::WrongKey)
        }
    }

    /// Creates a [`ImageNode`] component for the given atlas key, if available in any of the loaded Atlases.
    pub fn try_make_image_node<T: AsRef<str>>(&self, key: T) -> Result<ImageNode, RpackAtlasError> {
        if let Ok((atlas, image)) = self.get_atlas_data(key) {
            Ok(ImageNode::from_atlas_image(image, atlas))
        } else {
            Err(RpackAtlasError::WrongKey)
        }
    }
}

/// Plugin that provides support for rpack atlases.
///
/// # Example
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_rpack::prelude::*;
///
/// App::new()
///     .add_plugins((DefaultPlugins,RpackAssetPlugin))
///     .run();
/// ```
pub struct RpackAssetPlugin;

impl Plugin for RpackAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<super::AtlasAsset>();
        app.register_type::<RpackAtlasAsset>();
        app.init_asset::<RpackAtlasAsset>();
        app.init_asset_loader::<RpackAtlasAssetLoader>();
    }
}

/// Errors that can occur while loading or processing a `RpackAtlasAsset`.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum RpackAtlasAssetError {
    /// An [IO](std::io) Error that occured
    /// during parsing of a `.rpack.json` file.
    #[error("could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// An error that occurred while parsing the `.rpack.json` file into an asset structure.
    #[error("could not parse asset: {0}")]
    ParsingError(#[from] serde_json::Error),
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

/// The loader responsible for loading `RpackAtlasAsset` files from `.rpack.json` files.
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
            .unwrap_or(std::path::Path::new(""))
            .join(asset.filename);

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
