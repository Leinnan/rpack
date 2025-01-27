# Bevy_rPack

A Bevy plugin with support for the `rpack.json` atlases.

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
        .add_systems(Update, on_loaded)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Holder(asset_server.load("tilemap.rpack.json")));
    commands.spawn(Camera2d);
}

fn on_loaded(
    mut ev_asset: EventReader<AssetEvent<RpackAtlasAsset>>,
    atlases: RpackAtlases,
    mut commands: Commands,
) {
    for ev in ev_asset.read() {
        if !matches!(ev, AssetEvent::LoadedWithDependencies { id: _ }) {
            continue;
        }

        if let Ok(sprite) = atlases.try_make_sprite("agents/spaceAstronauts_005") {
            commands.spawn(Sprite {
                color: Color::linear_rgb(1.0, 0.0, 0.0),
                ..sprite
            });
        };
        if let Ok(image_node) = atlases.try_make_image_node("agents/spaceShips_006") {
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
