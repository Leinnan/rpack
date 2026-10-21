//! Displays the first frame from assets/ships.rpack_gen.json.
use bevy::prelude::*;
use bevy_rpack::prelude::*;

#[derive(Resource)]
struct GeneratedAtlas(Handle<RpackAtlasAsset>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RpackAssetPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, show_atlas)
        .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(GeneratedAtlas(server.load("ships.rpack_gen.json")));
}

fn show_atlas(
    mut commands: Commands,
    handle: Res<GeneratedAtlas>,
    atlases: Res<Assets<RpackAtlasAsset>>,
    mut shown: Local<bool>,
) {
    if *shown {
        return;
    }
    if let Some(atlas) = atlases.get(&handle.0) {
        if let Some(key) = atlas.files.keys().min() {
            commands.spawn(atlas.try_make_sprite(key).expect("key belongs to atlas"));
            *shown = true;
        }
    }
}
