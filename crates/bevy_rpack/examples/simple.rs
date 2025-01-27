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
