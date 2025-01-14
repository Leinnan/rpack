# Bevy_rPack

A Bevy plugin with support for the `rpack.json` atlases.

## Example

```rust
//! Simple example that loads the tilemap and once is loaded it creates a sprite with it.

use bevy::prelude::*;
use bevy_rpack::prelude::*;

#[allow(dead_code)]
#[derive(Resource)]
struct Holder(pub Handle<RpackAtlasAsset>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RpackAssetPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, on_loaded)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Holder(asset_server.load("tilemap.rpack.json")));
    commands.spawn(Camera2d);
}

fn on_loaded(
    mut ev_asset: EventReader<AssetEvent<RpackAtlasAsset>>,
    assets: Res<Assets<RpackAtlasAsset>>,
    mut commands: Commands,
) {
    for ev in ev_asset.read() {
        let AssetEvent::LoadedWithDependencies { id: _ } = ev else {
            continue;
        };

        if let Ok(sprite) = assets.try_make_sprite_from_atlas("agents/spaceAstronauts_005") {
            commands.spawn(Sprite {
                color: Color::linear_rgb(1.0, 0.0, 0.0),
                ..sprite
            });
        };
        if let Ok(image_node) = assets.try_make_image_node_from_atlas("agents/spaceShips_006") {
            commands.spawn(image_node);
        }
    }
}
```

## Licence

`bevy_rpack` is dual-licensed under MIT and Apache 2.0 at your option.

## Bevy compatibility table

Bevy version | Crate version
--- | ---
0.15 | 0.1
