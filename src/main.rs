use bevy::{prelude::*, render::camera::{Camera, OrthographicProjection}};

mod character;
use character::{Character, CharacterState, Direction};

struct Player;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_system.system())
        .add_system(animate_sprite_system.system())
        .add_system(move_sprite_system.system())
        .add_system(update_camera_system.system())
        .add_system(keyboard_input_system.system())
        .run();
}


fn keyboard_input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Character>,
) {
    if keyboard_input.just_pressed(KeyCode::W) {
        for mut character in query.iter_mut() {
            character.direction = Direction::North;
            character.state = CharacterState::Walking;
            character.velocity.x = 0.0;
            character.velocity.y = 1.0;
        }
    }
    if keyboard_input.just_pressed(KeyCode::A) {
        for mut character in query.iter_mut() {
            character.direction = Direction::West;
            character.state = CharacterState::Walking;
            character.velocity.x = -1.0;
            character.velocity.y = 0.0;
        }
    }
    if keyboard_input.just_pressed(KeyCode::S) {
        for mut character in query.iter_mut() {
            character.direction = Direction::South;
            character.state = CharacterState::Walking;
            character.velocity.x = 0.0;
            character.velocity.y = -1.0;
        }
    }
    if keyboard_input.just_pressed(KeyCode::D) {
        for mut character in query.iter_mut() {
            character.direction = Direction::East;
            character.state = CharacterState::Walking;
            character.velocity.x = 1.0;
            character.velocity.y = 0.0;
        }
    }
    if keyboard_input.just_released(KeyCode::W)
        || keyboard_input.just_released(KeyCode::A)
        || keyboard_input.just_released(KeyCode::S)
        || keyboard_input.just_released(KeyCode::D) {

        for mut character in query.iter_mut() {
            if !keyboard_input.pressed(KeyCode::W) && !keyboard_input.pressed(KeyCode::S) {
                character.velocity.y = 0.0;
            }
            if !keyboard_input.pressed(KeyCode::A) && !keyboard_input.pressed(KeyCode::D) {
                character.velocity.x = 0.0;
            }
            // disable animation if no longer moving
            if character.velocity.distance(Vec3::zero()) < 0.01
            {
                character.make_idle();
            }

        }
    }
}

const PLAYER_WIDTH: f32 = 31.0;
const PLAYER_HEIGHT: f32 = 32.0;

fn setup_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("sprites/character.png");
    let bg_handle = asset_server.load("backgrounds/world_map_wallpaper.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle,
                                                Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT), 8, 16);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands
        .spawn(Camera2dBundle::default())
        .spawn(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_scale(Vec3::splat(4.0))
                        .mul_transform(Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))),
            ..Default::default()
        })
        .with(Timer::from_seconds(0.1, true))
        .with(Character::default())
        .with(Player {})
        // background
        .spawn(SpriteBundle {
            material: materials.add(bg_handle.into()),
            transform: Transform::from_scale(Vec3::splat(2.0)),
            ..Default::default()
        });
}

fn move_sprite_system(
    time: Res<Time>,
    mut query: Query<(&Character, &mut Transform)>,
) {
    for (character, mut transform) in query.iter_mut() {
        transform.translation = transform.translation + character.velocity * time.delta_seconds() * character.movement_speed;
    }
}

fn bounding_box(translation: Vec3, size: Vec2) -> Rect<f32> {
    Rect {
        left: translation.x,
        right: translation.x + size.x,
        top: translation.y,
        bottom: translation.y + size.y,
    }
}

fn viewport(camera_transform: &Transform, projection: &OrthographicProjection) -> Rect<f32> {
    let translation = camera_transform.translation;
    Rect {
        left: projection.left + translation.x,
        right: projection.right + translation.x,
        top: projection.top + translation.y,
        bottom: projection.bottom + translation.y,
    }
}

// Returns true if r1 is completely contained withing r2.
fn is_rect_completely_inside(r1: &Rect<f32>, r2: &Rect<f32>) -> bool {
    r1.left > r2.left && r1.right < r2.right &&
    r1.bottom > r2.bottom && r1.top < r2.top
}

fn update_camera_system(
    mut player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<Camera>>,
) {
    for player_transform in player_query.iter_mut() {
        // Is sprite in view frame?
        // println!("player {:?}", player_transform.translation);
        let char_translation = player_transform.translation;
        // TODO: Use player scaling.
        let char_rect = bounding_box(char_translation, Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT));
        // println!("char_rect {:?}", char_rect);
        for (mut camera_transform, projection) in camera_query.iter_mut() {
            // println!("projection {:?}", projection);
            let camera_rect = viewport(&camera_transform, projection);
            // println!("camera_rect {:?}", camera_rect);
            let is_player_in_view = is_rect_completely_inside(&char_rect, &camera_rect);
            if !is_player_in_view {
                camera_transform.translation = player_transform.translation;
            }
        }
    }
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
            let num_cells_per_row = 8;
            let (num_cells_in_animation, start_index) = match character.state {
                CharacterState::Idle    => (1, row * num_cells_per_row + 1),
                CharacterState::Walking => (3, row * num_cells_per_row),
            };
            let index_in_animation = (character.animation_index + 1) % num_cells_in_animation;
            character.animation_index = index_in_animation;
            sprite.index = ((start_index + index_in_animation as usize) % total_num_cells) as u32;
        }
    }
}
