pub use rpack_core::{
    ImageFile, SaveImageFormat, Spritesheet, SpritesheetBuildConfig, SpritesheetError,
    TilemapGenerationConfig, formats, get_common_prefix, packing,
};
#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
use std::{io::Write, path::Path};

#[deprecated(note = "Use packing::place for dimension-only atlas placement")]
pub mod packer;
pub mod saving;

/// Writes the atlas described by a config to disk.
#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
pub trait GenerateAtlas {
    /// Packs the config's images, then saves the atlas image and its `.rpack.json`.
    fn generate(&self) -> anyhow::Result<()>;
}

#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
impl GenerateAtlas for TilemapGenerationConfig {
    fn generate(&self) -> anyhow::Result<()> {
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
