use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_system.system())
        .add_system(animate_sprite_system.system())
        .add_system(keyboard_input_system.system())
        .run();
}

struct Character {
    direction: Direction,
    animation_index: u32,
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    North,
    South,
    East,
    West,
}

fn keyboard_input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Character>,
) {
    if keyboard_input.just_pressed(KeyCode::W) {
        for mut character in query.iter_mut() {
            character.direction = Direction::North;
        }
    }
    if keyboard_input.just_pressed(KeyCode::A) {
        for mut character in query.iter_mut() {
            character.direction = Direction::West;
        }
    }
    if keyboard_input.just_pressed(KeyCode::S) {
        for mut character in query.iter_mut() {
            character.direction = Direction::South;
        }
    }
    if keyboard_input.just_pressed(KeyCode::D) {
        for mut character in query.iter_mut() {
            character.direction = Direction::East;
        }
    }
}

fn setup_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("sprites/character.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle,
                                                Vec2::new(31.0, 32.0), 8, 16);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands
        .spawn(Camera2dBundle::default())
        .spawn(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_scale(Vec3::splat(4.0)),
            ..Default::default()
        })
        .with(Timer::from_seconds(0.1, true))
        .with(Character { direction: Direction::South, animation_index: 0 });
}

fn animate_sprite_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &Handle<TextureAtlas>, &mut Character)>
) {
    for (mut timer, mut sprite, texture_atlas_handle, mut character) in query.iter_mut() {
        timer.tick(time.delta_seconds());
        if timer.finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).expect("should have found texture atlas handle");
            let total_num_cells = texture_atlas.textures.len();
            let row = match character.direction {
                Direction::North => 3,
                Direction::South => 0,
                Direction::East => 2,
                Direction::West => 1,
            };
            let num_cells_in_animation = 3;
            let index_in_animation = (character.animation_index + 1) % num_cells_in_animation;
            character.animation_index = index_in_animation;
            let num_cells_per_row = 8;
            let start_index = row * num_cells_per_row;
            sprite.index = ((start_index + index_in_animation as usize) % total_num_cells) as u32;
        }
    }
}
