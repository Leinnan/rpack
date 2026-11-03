//! Packs the Bevy example config and compares the result with recorded output.
//!
//! Guards the atlas layout and JSON format against refactors. After an intended
//! change, re-record with `RPACK_UPDATE_GOLDEN=1 cargo test -p rpack_cli --test golden`.

use rpack_cli::{ImageFile, Spritesheet, TilemapGenerationConfig, packing};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// FNV-1a, stable across platforms and dependency updates.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[test]
fn example_config_output_is_unchanged() {
    let config_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy_rpack/example_config.rpack_gen.json");
    let config = TilemapGenerationConfig::read_from_file(config_path).unwrap();
    let paths = config.try_file_paths().unwrap();
    let keys = packing::frame_keys(&paths).unwrap();
    let images: Vec<ImageFile> = paths
        .iter()
        .zip(keys)
        .map(|(path, id)| ImageFile {
            id,
            image: image::open(path).unwrap(),
        })
        .collect();
    let sheet = Spritesheet::build(&config, &images, "tilemap.png").unwrap();

    let mut json = sheet.atlas_asset_json.clone();
    // The app version changes with every release and is not part of the layout.
    json["metadata"]["app_version"] = Value::String("<version>".to_owned());
    let json = serde_json::to_string_pretty(&json).unwrap();
    let pixels = format!(
        "{:016x}\n",
        fnv1a(sheet.image_data.to_rgba8().as_raw().as_slice())
    );

    let json_path = golden_dir().join("tilemap.rpack.json");
    let pixels_path = golden_dir().join("tilemap.pixels.fnv1a");
    if std::env::var_os("RPACK_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(golden_dir()).unwrap();
        std::fs::write(&json_path, &json).unwrap();
        std::fs::write(&pixels_path, &pixels).unwrap();
    }
    assert_eq!(json, std::fs::read_to_string(json_path).unwrap());
    assert_eq!(pixels, std::fs::read_to_string(pixels_path).unwrap());
}
