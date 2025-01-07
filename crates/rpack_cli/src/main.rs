use std::io::Write;

use clap::Parser;
use rpack_cli::{ImageFile, Spritesheet};

/// Build rpack tilemaps with ease
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the tilemap to build, when no value is provided uses 'tilemap'
    #[arg(short, long)]
    name: Option<String>,
    /// size of the tilemap, default: 512
    #[arg(long)]
    size: Option<u32>,
}

fn main() {
    let args = Args::parse();
    let name = args.name.unwrap_or("tilemap".to_owned());
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
        name.clone(),
    )
    .expect("Failed to build spritesheet");

    let mut file = std::fs::File::create(format!("{}.png", name)).unwrap();
    let write_result = file.write_all(&spritesheet.image_data);
    if write_result.is_err() {
        eprintln!(
            "Could not make atlas, error: {:?}",
            write_result.unwrap_err()
        );
    } else {
        println!("Output texture stored in {:?}", file);
    }
    let json = serde_json::to_string_pretty(&spritesheet.atlas_asset_json).unwrap();
    let mut file = std::fs::File::create(format!("{}.rpack.json", name)).unwrap();
    let write_result = file.write_all(&json.as_bytes());
    if write_result.is_err() {
        eprintln!(
            "Could not make atlas, error: {:?}",
            write_result.unwrap_err()
        );
    } else {
        println!("Output data stored in {:?}", file);
    }
}
