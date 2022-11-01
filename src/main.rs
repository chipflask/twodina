use anyhow::Result;
use bevy::prelude::*;

mod camera;
mod core;
mod motion;
mod players;

use crate::camera::update_camera_system;
use crate::core::character::{Character};
use crate::core::input::keyboard_input_system;
use crate::motion::{AnimateTimer, animate_sprite_system, move_sprite_system};
use crate::players::Player;

fn main() -> Result<()> {
    let config = core::config::load_asset_config("app.toml")?;

    App::new()
        .insert_resource(config)
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_system)
        .add_system(animate_sprite_system)
        .add_system(move_sprite_system)
        .add_system(update_camera_system)
        .add_system(keyboard_input_system)
        .run();

    Ok(())
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let player = Player::default();
    let texture_handle = asset_server.load("sprites/character.png");
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(player.width, player.height),
        8,
        16,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands.spawn().insert_bundle(Camera2dBundle::default());
    commands
        .spawn()
        .insert_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_scale(Vec3::splat(4.0)).mul_transform(
                Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)),
            ),
            ..Default::default()
        })
        .insert(AnimateTimer {
            timer: Timer::from_seconds(0.1, true),
        })
        .insert(Character::default())
        .insert(player);
    // background
    commands.spawn().insert_bundle(SpriteBundle {
        texture: asset_server.load("backgrounds/world_map_wallpaper.png"),
        transform: Transform::from_scale(Vec3::splat(2.0)),
        ..Default::default()
    });
}
