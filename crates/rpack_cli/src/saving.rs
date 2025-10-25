use image::{DynamicImage, RgbaImage};
use std::path::Path;

use crate::formats::SaveImageFormat;

pub trait SaveableImage {
    fn save_with_format_autodetection<R: AsRef<Path>>(&self, path: R) -> anyhow::Result<()> {
        let output_path = path.as_ref().to_owned();
        let format = SaveImageFormat::from_path(&output_path);
        match format {
            None => {
                let output_extension = output_path
                    .extension()
                    .map_or(String::from("png"), |e| e.to_string_lossy().to_string());
                let Some(output_format) = image::ImageFormat::from_extension(&output_extension)
                else {
                    anyhow::bail!("Unsupported output format");
                };
                self.to_rgba8()
                    .save_with_format(output_path, output_format)?;
            }
            Some(format) => match format {
                SaveImageFormat::Png => {
                    self.to_rgba8()
                        .save_with_format(output_path, image::ImageFormat::Png)?;
                }
                SaveImageFormat::Basis => {
                    #[cfg(feature = "basis")]
                    self.save_as_basis(&output_path)?;
                    #[cfg(not(feature = "basis"))]
                    anyhow::bail!(
                        "Program is compiled without support for basis. Compile it yourself with feature `basis` enabled."
                    );
                }
                SaveImageFormat::Dds => {
                    #[cfg(feature = "dds")]
                    self.save_as_dds(&output_path)?;
                    #[cfg(not(feature = "basis"))]
                    anyhow::bail!(
                        "Program is compiled without support for basis. Compile it yourself with feature `basis` enabled."
                    );
                }
            },
        }
        Ok(())
    }

    fn to_rgba8(&self) -> RgbaImage;

    #[cfg(all(feature = "basis", not(target_arch = "wasm32")))]
    fn save_as_basis(&self, output_path: impl AsRef<Path>) -> anyhow::Result<()> {
        use basis_universal::{BasisTextureFormat, Compressor, Transcoder};
        use image::EncodableLayout;

        let rgba_image = self.to_rgba8();

        let channel_count = 4;
        let (pixel_width, pixel_height) = rgba_image.dimensions();
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

    #[cfg(all(feature = "dds", not(target_arch = "wasm32")))]
    fn save_as_dds<R>(&self, output_path: R) -> anyhow::Result<()>
    where
        R: AsRef<Path>,
    {
        let rgba_image = self.to_rgba8();

        let dds = image_dds::dds_from_image(
            &rgba_image,
            image_dds::ImageFormat::Rgba8Unorm,
            image_dds::Quality::Fast,
            image_dds::Mipmaps::GeneratedAutomatic,
        )?;

        let mut writer = std::io::BufWriter::new(std::fs::File::create(output_path.as_ref())?);
        dds.write(&mut writer)?;
        Ok(())
    }
}

impl SaveableImage for DynamicImage {
    fn to_rgba8(&self) -> RgbaImage {
        self.to_rgba8()
    }
}
