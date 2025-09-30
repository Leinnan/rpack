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
    commands.spawn(Camera2d);
    commands.insert_resource(Holder(asset_server.load("tilemap.rpack.json")));
}

fn atlas_loaded(
    mut ev_asset: MessageReader<AssetEvent<RpackAtlasAsset>>,
    atlases: RpackAtlases,
    mut commands: Commands,
) {
    if !ev_asset
        .read()
        .any(|ev| matches!(ev, AssetEvent::LoadedWithDependencies { id: _ }))
    {
        return;
    }
    info!("Atlas loaded");
    match atlases.try_make_sprite("agents/spaceAstronauts_005") {
        Ok(sprite) => {
            commands.spawn((sprite, Transform::from_xyz(0.0, 20.0, 0.0)));
        }
        Err(e) => error!("Error loading sprite: {}", e),
    }
    match atlases.try_make_image_node("agents/spaceShips_006") {
        Ok(image_node) => {
            commands.spawn(image_node);
        }
        Err(e) => error!("Error loading sprite for image node: {}", e),
    }
}
