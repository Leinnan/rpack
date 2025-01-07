use bevy_rpack::{AtlasFrame, SerializableRect};
use image::DynamicImage;
use serde_json::Value;
use std::{io::Cursor, path::PathBuf};
use texture_packer::{importer::ImageImporter, TexturePacker, TexturePackerConfig};

#[derive(Clone)]
pub struct Spritesheet {
    pub image_data: Vec<u8>,
    pub atlas_asset: bevy_rpack::AtlasAsset,
    pub atlas_asset_json: Value,
}

#[derive(Clone)]
pub struct ImageFile {
    pub id: String,
    pub image: DynamicImage,
}

impl ImageFile {
    pub fn at_path<P>(path: &PathBuf, id: P) -> Option<ImageFile>
    where
        P: AsRef<str>,
    {
        if let Ok(image) = ImageImporter::import_from_file(&path) {
            Some(ImageFile {
                image,
                id: id.as_ref().to_owned().replace("\\", "/"),
            })
        } else {
            None
        }
    }
}

impl Spritesheet {
    pub fn build(
        config: TexturePackerConfig,
        images: &[ImageFile],
        name: String,
    ) -> Result<Self, String> {
        let mut packer = TexturePacker::new_skyline(config);
        for image in images.iter() {
            if !packer.can_pack(&image.image) {
                return Err(format!(
                    "Consider making atlas bigger. Could not make atlas, failed on: {}",
                    image.id
                ));
            }
            if let Err(err) = packer.pack_own(&image.id, image.image.clone()) {
                return Err(format!(
                    "Could not make atlas, failed on: {}, {:?}",
                    image.id, err
                ));
            }
        }
        let mut out_vec = vec![];
        let exported_image =
            texture_packer::exporter::ImageExporter::export(&packer, None).unwrap();
        let mut img = image::DynamicImage::new_rgba8(config.max_width, config.max_height);
        image::imageops::overlay(&mut img, &exported_image, 0, 0);

        img.write_to(&mut Cursor::new(&mut out_vec), image::ImageFormat::Png)
            .unwrap();
        let atlas_asset = bevy_rpack::AtlasAsset {
            size: [img.width(), img.height()],
            name,
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
        let Ok(atlas_asset_json) = serde_json::to_value(&atlas_asset) else {
            return Err("Failed to deserialize".to_owned());
        };

        Ok(Spritesheet {
            image_data: out_vec.clone(),
            atlas_asset,
            atlas_asset_json,
        })
    }
}
