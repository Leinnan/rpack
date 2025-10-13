# rpack [![Build Status](https://github.com/Leinnan/rpack/workflows/CI/badge.svg)](https://github.com/Leinnan/rpack/actions?workflow=CI)


Create tilemaps in seconds!

This repository contains few projects that together make a fully functional solution for generating tilemaps alongside integration to the Bevy game engine.

## Bevy rPack

[![Crates.io](https://img.shields.io/crates/v/bevy_rpack)](https://crates.io/crates/bevy_rpack)
[![Documentation](https://docs.rs/bevy_rpack/badge.svg)](https://docs.rs/bevy_rpack)

A Bevy plugin with support for the `rpack.json` atlases.

More info available at [crates/bevy_rpack](https://github.com/Leinnan/rpack/tree/master/crates/bevy_rpack).

Repository contains example how to use plugin in Bevy.

## rPack CLI

[![Crates.io](https://img.shields.io/crates/v/rpack_cli)](https://crates.io/crates/rpack_cli)
[![Documentation](https://docs.rs/rpack_cli/badge.svg)](https://docs.rs/rpack_cli)

Command line interface for generating tilemaps.

```sh
Build rpack tilemaps with ease

Usage: rpack_cli <COMMAND>

Commands:
  generate              Generates a tilemap
  config-create         Creates a tilemap generation config
  generate-from-config  Generates a tilemap from config
  help                  Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Available at [crates/rpack_cli](https://github.com/Leinnan/rpack/tree/master/crates/rpack_cli).

### Installation

Only with Cargo (Rust package manager) at the moment:

```sh
cargo install rpack_cli
```

## rPack egui

[![Crates.io](https://img.shields.io/crates/v/rpack_egui)](https://crates.io/crates/rpack_egui)

A both desktop and web frontend for generating tilemaps. Just drag and drop images into the program and generate tilemaps.

To open it in browser click one of the icons below:

[![Itch.io](https://img.shields.io/badge/Itch-%23FF0B34.svg?style=for-the-badge&logo=Itch.io&logoColor=white)](https://mevlyshkin.itch.io/rpack)
[![Github Pages](https://img.shields.io/badge/github%20pages-121013?style=for-the-badge&logo=github&logoColor=white)](http://rpack.mevlyshkin.com/)

<img width="1103" height="754" alt="Rpack EGUI 0.3.0" src="https://github.com/user-attachments/assets/f9fa09b0-8634-43f8-91a2-2031c5ae6026" />


Available at [crates/rpack_egui](https://github.com/Leinnan/rpack/tree/master/crates/rpack_egui).


## Used formats

rpack tools provides and work with two json based files.

### Atlas files

Tilemaps are using `.rpack.json` extension.

Fields:

- `size`: two element array- width and height of the tilemap
- `filename`: string- path to the atlas image file, relative to the config file
- `frames`: array- contain info about each frame in tilemap, contains `key` string field and `frame` field that is made up from fields:
  - `h`- image height
  - `w`- image width
  - `x`- x start pos of the image in the tilemap
  - `y`- y start pos of the image in the tilemap

Example:

```json
{
  "filename": "tilemap.png",
  "frames": [
    {
      "frame": {
        "h": 42,
        "w": 42,
        "x": 418,
        "y": 66
      },
      "key": "tiles/ship/spaceBuilding_001"
    },
    {
      "frame": {
        "h": 44,
        "w": 34,
        "x": 2,
        "y": 2
      },
      "key": "tiles/agents/spaceAstronauts_004"
    },
  ],
  "metadata": {
    "app": "rpack",
    "app_version": "0.3.0",
    "format_version": 1
  },
  "size": [
    512,
    512
  ]
}
```

### Generation config files

Config files are using `.rpack_gen.json` extension.

Fields:

- `output_path`: string- path relative to the config, without extension, this is where tilemap image and `.rpack.json` config file are going to be saved
- `asset_patterns`: array of strings- search patterns for images to be included, relative paths to the config
- `format`: optional(defaults to `Png`), format of the tilemap image, currently supported values: `Png`, `Dds`
- `size`: optional(defaults to `2048`), size of the tilemap image
- `texture_padding`: optional(defaults to `2`), size of the padding between frames in pixel
- `border_padding`: optional(defaults to `0`), size of the padding on the outer edge of the packed image in pixel
- `metadata`: optional, struct containing metadata about the program used to generate the tilemap and version number, stored for the future in case of future breaking changes

Example:

```json
{
  "asset_patterns": [
    "tiles/agents/*",
    "tiles/effects/*",
    "tiles/missiles/*",
    "tiles/ship/spaceBuilding_00*",
    "tiles/ship/spaceBuilding_01*"
  ],
  "output_path": "assets/tilemap",
  "format": "Png",
  "size": 512,
  "texture_padding": 2,
  "border_padding": 2,
  "size": [
    512,
    512
  ]
}
```
