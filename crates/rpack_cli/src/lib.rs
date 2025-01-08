use bevy_rpack::{AtlasFrame, SerializableRect};
use image::DynamicImage;
use serde_json::Value;
use std::path::Path;
use texture_packer::{importer::ImageImporter, TexturePacker, TexturePackerConfig};

#[derive(Clone)]
pub struct Spritesheet {
    pub image_data: DynamicImage,
    pub atlas_asset: bevy_rpack::AtlasAsset,
    pub atlas_asset_json: Value,
}

#[derive(Clone)]
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

impl Spritesheet {
    pub fn build<P>(
        config: TexturePackerConfig,
        images: &[ImageFile],
        filename: P,
    ) -> Result<Self, String>
    where
        P: AsRef<str>,
    {
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
        let Ok(image_data) = texture_packer::exporter::ImageExporter::export(&packer, None) else {
            return Err("Failed to export image".to_owned());
        };

        let atlas_asset = bevy_rpack::AtlasAsset {
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
        let Ok(atlas_asset_json) = serde_json::to_value(&atlas_asset) else {
            return Err("Failed to deserialize".to_owned());
        };

        Ok(Spritesheet {
            image_data,
            atlas_asset,
            atlas_asset_json,
        })
    }

    #[cfg(all(feature = "dds", not(target_arch = "wasm32")))]
    pub fn save_as_dds<R>(&self, output_path: R)
    where
        R: AsRef<str>,
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
    pub fn save_as_basis<R>(&self, output_path: R)
    where
        R: AsRef<str>,
    {
        use basis_universal::{
            BasisTextureFormat, Compressor, TranscodeParameters, Transcoder,
            TranscoderTextureFormat,
        };
        use image::{EncodableLayout, GenericImageView};

        let rgba_image = self.image_data.to_rgba8();

        let channel_count = 4;
        let (pixel_width, pixel_height) = self.image_data.dimensions();
        let mut compressor_params = basis_universal::CompressorParams::new();
        compressor_params.set_generate_mipmaps(true);
        compressor_params.set_basis_format(BasisTextureFormat::UASTC4x4);
        compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);
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
            compressor.process().unwrap();
            let t1 = std::time::Instant::now();
            t1 - t0
        };

        // You could write it to disk like this
        let basis_file = compressor.basis_file();
        // std::fs::write("example_encoded_image.basis", basis_file).unwrap();

        let mut transcoder = Transcoder::new();
        let mip_level_count = transcoder.image_level_count(basis_file, 0);
        println!(
            "Compressed {} mip levels to {} total bytes in {} ms",
            mip_level_count,
            compressor.basis_file_size(),
            compression_time.as_secs_f64() * 1000.0
        );

        let userdata = transcoder.user_data(basis_file).unwrap();
        println!("Basis file has user data {:?}", userdata);

        //
        // Now lets transcode it back to raw images
        //
        transcoder.prepare_transcoding(basis_file).unwrap();

        let t0 = std::time::Instant::now();
        let result = transcoder
            .transcode_image_level(
                basis_file,
                TranscoderTextureFormat::ASTC_4x4_RGBA,
                TranscodeParameters {
                    image_index: 0,
                    level_index: 0,
                    ..Default::default()
                },
            )
            .unwrap();
        let t1 = std::time::Instant::now();

        println!(
            "Transcoded mip level 0 to ASTC_4x4_RGBA: {} bytes {} ms",
            result.len(),
            (t1 - t0).as_secs_f64() * 1000.0
        );

        let t0 = std::time::Instant::now();
        let result = transcoder
            .transcode_image_level(
                basis_file,
                TranscoderTextureFormat::RGBA32,
                TranscodeParameters {
                    image_index: 0,
                    level_index: 0,
                    ..Default::default()
                },
            )
            .unwrap();
        let t1 = std::time::Instant::now();

        println!(
            "Transcoded mip level 0 to RGBA32: {} bytes {} ms",
            result.len(),
            (t1 - t0).as_secs_f64() * 1000.0
        );

        transcoder.end_transcoding();

        let description = transcoder
            .image_level_description(basis_file, 0, 0)
            .unwrap();
        let image = image::RgbaImage::from_raw(
            description.original_width,
            description.original_height,
            result,
        )
        .unwrap();
        // TODO THIS DOESNT WORK, NEED TO FIX THIS
        image
            .save_with_format(output_path.as_ref(), image::ImageFormat::Png)
            .unwrap();
    }
}
