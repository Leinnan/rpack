# Bevy_rPack

A Bevy plugin with support for the `rpack.json` atlases.

## Getting the rpack.json atlases

Generating a rpack atlas can be done in two ways:
- using `rpack_egui` application at [itch.io](https://mevlyshkin.itch.io/rpack). Just drop in files into program, specify settings and it should be possible to download both the json and image file for atlas.
- using `rpack_cli` CLI tool as described below.

### rpack_cli use guide

First install the tool:

```sh
cargo install rpack_cli
```

Guide assumptions:
- Commands are executed from the root of the Bevy project.
- Images used to generate the atlas are stored in subdirectories within the `tiles` directory of the Bevy project.
- Game assets are stored in the `assets` directory.

In order to generate a tilemap execute this command:
```sh
rpack_cli generate --source-paths "tiles/**/*" --size 1024 assets/game_tilemap
```
It is also possible to generate a config file for `rpack_cli` and then use it to generate a tilemaps:
```sh
rpack_cli config-create --source-paths "tiles/**/*" --size 1024 --output-path assets/tilemap tiles_config
rpack_cli tiles_config.rpack_gen.json
```
Both ways result in creation of two files:
- `assets/tilemap.png` containing atlas image
- `assets/tilemap.rpack.json` containing atlas image frames data used by bevy plugin.

Second way generated additional `tiles_config.rpack_gen.json` file that provides settings for the CLI tool.

With those files generated it should be possible to use atlases like in the example from `Example` docs section.

#### Bonus- generating tilemaps on build/runtime

Since `rpack_cli` is a rust library it can be called easily from Bevy application. 
It can be useful for building special version of the game for artists in the team so they can run the game and be sure that they don't forget to rebuild atlases after doing changes in graphics.

In order to achieve that add this dependency to the project:
```toml
rpack_cli = "0.1"
```
Then in `main.rs` just before `App::new()` add something like this:

```rust
rpack_cli::TilemapGenerationConfig::read_from_file("example_config.rpack_gen.json")
    .expect("Failed to read config")
    .generate()
    .expect("Failed to generate tilemap");
```

Then on each run of the game it will regenerate the atlas before running the game. 

> Disclaimer: It should be used carefully and in most cases be called only in development builds on desktop platforms.

## Example

```rust
use bevy::prelude::*;
use bevy_rpack::prelude::*;

#[allow(dead_code)]
#[derive(Resource)]
struct Holder(pub Handle<RpackAtlasAsset>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RpackAssetPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, atlas_loaded)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Holder(asset_server.load("tilemap.rpack.json")));
    commands.spawn(Camera2d);
}

fn atlas_loaded(
    mut ev_asset: EventReader<AssetEvent<RpackAtlasAsset>>,
    atlases: RpackAtlases,
    mut commands: Commands,
) {
    if !ev_asset
        .read()
        .any(|ev| matches!(ev, AssetEvent::LoadedWithDependencies { id: _ }))
    {
        return;
    }
    if let Ok(sprite) = atlases.try_make_sprite("agents/spaceAstronauts_005") {
        commands.spawn(sprite);
    };
    if let Ok(image_node) = atlases.try_make_image_node("agents/spaceShips_006") {
        commands.spawn(image_node);
    }
}

```

## Licence

`bevy_rpack` is dual-licensed under MIT and Apache 2.0 at your option.

## Bevy compatibility table

Bevy version | Crate version
--- | ---
0.15 | 0.1
