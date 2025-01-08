use std::{fmt::Display, io::Write, path::Path};

use clap::{Parser, ValueEnum};
use rpack_cli::{ImageFile, Spritesheet};

#[derive(Clone, Debug, Default, Copy, ValueEnum)]
pub enum SaveImageFormat {
    #[default]
    Png,
    Dds,
}

impl Display for SaveImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveImageFormat::Png => f.write_str(".png"),
            SaveImageFormat::Dds => f.write_str(".dds"),
        }
    }
}

impl From<SaveImageFormat> for image::ImageFormat {
    fn from(val: SaveImageFormat) -> Self {
        match val {
            SaveImageFormat::Png => image::ImageFormat::Png,
            SaveImageFormat::Dds => image::ImageFormat::Dds,
        }
    }
}

/// Build rpack tilemaps with ease
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the tilemap to build, when no value is provided uses 'tilemap'
    #[arg(action)]
    name: Option<String>,
    /// size of the tilemap, default: 512
    #[arg(long)]
    size: Option<u32>,
    /// Image format
    #[clap(short, long)]
    format: Option<SaveImageFormat>,
}

fn main() {
    let args = Args::parse();
    let name = args.name.unwrap_or("tilemap".to_owned());
    let format = args.format.unwrap_or_default();
    let atlas_filename = format!("{}{}", name, format);
    let atlas_json_filename = format!("{}.png", name);
    let size = args.size.unwrap_or(512);

    let images: Vec<ImageFile> = glob::glob("**/*png")
        .expect("Failed to find the png files")
        .flatten()
        .flat_map(|f| ImageFile::at_path(&f, f.to_str().unwrap_or_default()))
        .collect();
    let spritesheet = Spritesheet::build(
        texture_packer::TexturePackerConfig {
            max_width: size,
            max_height: size,
            allow_rotation: false,
            force_max_dimensions: true,
            border_padding: 2,
            texture_padding: 2,
            texture_extrusion: 2,
            trim: false,
            texture_outlines: false,
        },
        &images,
        &atlas_filename,
    )
    .expect("Failed to build spritesheet");

    if Path::new(&atlas_json_filename).exists() {
        std::fs::remove_file(&atlas_json_filename).expect("Could not remove the old file");
    }
    if Path::new(&atlas_filename).exists() {
        std::fs::remove_file(&atlas_filename).expect("Could not remove the old file");
    }
    match format {
        SaveImageFormat::Dds => {
            #[cfg(feature = "dds")]
            spritesheet.save_as_dds(&atlas_filename);
            #[cfg(not(feature = "dds"))]
            panic!("Program is compiled without support for dds. Compile it yourself with feature `dds` enabled.");
        }
        f => {
            let write_result = spritesheet
                .image_data
                .save_with_format(&atlas_filename, f.into());

            if write_result.is_err() {
                eprintln!(
                    "Could not make atlas, error: {:?}",
                    write_result.unwrap_err()
                );
            } else {
                println!("Output texture stored in {}", atlas_json_filename);
            }
        }
    }
    let json = serde_json::to_string_pretty(&spritesheet.atlas_asset_json).unwrap();
    let mut file = std::fs::File::create(format!("{}.rpack.json", name)).unwrap();
    let write_result = file.write_all(json.as_bytes());
    if write_result.is_err() {
        eprintln!(
            "Could not make atlas, error: {:?}",
            write_result.unwrap_err()
        );
    } else {
        println!("Output data stored in {:?}", file);
    }
}
