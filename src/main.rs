use std::convert::TryFrom;

use bevy::{prelude::*, render::camera::OrthographicProjection};
use bevy_tiled_prototype::{TiledMapCenter, TiledMapComponents, TiledMapPlugin};

mod character;
use character::{AnimatedSprite, Character, CharacterState, Direction, VELOCITY_EPSILON};
mod input;
use input::{Action, InputActionSet};

const NUM_PLAYERS: u32 = 2;

struct Player {
    id: u32,
}

struct PlayerPositionDisplay {
    player_id: u32,
}

// We have multiple cameras, so this one marks the camera that follows the
// player.
struct PlayerCamera;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(TiledMapPlugin)
        .add_plugin(input::InputActionPlugin::default())
        .add_startup_system(setup_system.system())
        .add_system(animate_sprite_system.system())
        .add_system(move_sprite_system.system())
        .add_system(update_camera_system.system())
        .add_system(handle_input_system.system())
        .add_system(position_display_system.system())
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .run();
}


fn handle_input_system(
    input_actions: Res<InputActionSet>,
    mut query: Query<(&mut Character, &Player, Option<&mut AnimatedSprite>)>,
) {
    for (mut character, player, animated_sprite_option) in query.iter_mut() {
        let mut new_direction = None;
        let mut new_velocity = Vec2::zero();
        let mut new_state = CharacterState::Idle;
        if input_actions.is_active(Action::Up, player.id) {
            new_direction = Some(Direction::North);
            new_velocity.y = 1.0;
            new_state = CharacterState::Walking;
        }
        if input_actions.is_active(Action::Down, player.id) {
            new_direction = Some(Direction::South);
            new_velocity.y = -1.0;
            new_state = CharacterState::Walking;
        }

        // Favor facing left or right when two directions are pressed simultaneously
        // by checking left/right after up/down.
        if input_actions.is_active(Action::Left, player.id) {
            new_direction = Some(Direction::West);
            new_velocity.x = -1.0;
            new_state = CharacterState::Walking;
        }
        if input_actions.is_active(Action::Right, player.id) {
            new_direction = Some(Direction::East);
            new_velocity.x = 1.0;
            new_state = CharacterState::Walking;
        }

        // If the user is pressing two directions at once, go diagonally with
        // unit velocity.
        if !new_velocity.abs_diff_eq(Vec2::zero(), VELOCITY_EPSILON) {
            new_velocity = new_velocity.normalize();
        }

        if let Some(direction) = new_direction {
            character.direction = direction;
        }
        character.velocity.x = new_velocity.x;
        character.velocity.y = new_velocity.y;
        // Don't modify z if the character has a z velocity for some reason.

        let old_state = character.state;
        if old_state != new_state {
            // We're transitioning to a new state.
            match new_state {
                CharacterState::Idle => {
                    character.make_idle();
                    if let Some(mut animated_sprite) = animated_sprite_option {
                        animated_sprite.reset();
                    }
                }
                CharacterState::Walking => {
                    if let Some(mut animated_sprite) = animated_sprite_option {
                        // Reset immediately to frame 1 so that the character looks like it starts
                        // walking when you press the key, not sliding until the next animation
                        // frame.  The fact that it's index 1 is just because of how our sprites
                        // are made, with the idle frame at index 1.
                        animated_sprite.reset_immediately(1);
                    }
                }
            }
        }
        character.state = new_state;
    }
}

const PLAYER_WIDTH: f32 = 31.0;
const PLAYER_HEIGHT: f32 = 32.0;

fn setup_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    for i in 0..NUM_PLAYERS {
        let texture_handle = asset_server.load(format!("sprites/character{}.png", i + 1).as_str());
        let texture_atlas = TextureAtlas::from_grid(texture_handle,
                                                    Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT), 8, 16);
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        commands
            .spawn(Camera2dBundle::default())
            .with(PlayerCamera {})
            .spawn(CameraUiBundle::default())
            .spawn(SpriteSheetBundle {
                texture_atlas: texture_atlas_handle,
                transform: Transform::from_scale(Vec3::splat(4.0))
                            .mul_transform(Transform::from_translation(Vec3::new(PLAYER_WIDTH * i as f32 + 20.0, 0.0, 5.0))),
                ..Default::default()
            })
            .with(AnimatedSprite::with_frame_seconds(0.1))
            .with(Character::default())
            .with(Player { id: i })
            .spawn(TextBundle {
                text: Text {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    value: "Position:".to_string(),
                    style: TextStyle {
                        color: Color::rgb(0.7, 0.7, 0.7),
                        font_size: 24.0,
                        ..Default::default()
                    },
                },
                style: Style {
                    position_type: PositionType::Absolute,
                    position: Rect {
                        top: Val::Px(5.0 + i as f32 * 20.0),
                        left: Val::Px(5.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .with(PlayerPositionDisplay { player_id: i});
    }
    // Map
    commands
        .spawn(TiledMapComponents {
            map_asset: asset_server.load("maps/ortho_map.tmx"),
            center: TiledMapCenter(true),
            origin: Transform::from_scale(Vec3::new(4.0, 4.0, 1.0)),
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
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<PlayerCamera>>,
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
    mut query: Query<(&mut TextureAtlasSprite, &Handle<TextureAtlas>, &mut AnimatedSprite, Option<&Character>)>
) {
    for (mut sprite, texture_atlas_handle, mut animated_sprite, character_option) in query.iter_mut() {
        animated_sprite.timer.tick(time.delta_seconds());
        if animated_sprite.needs_paint || animated_sprite.timer.finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).expect("should have found texture atlas handle");
            let total_num_cells = texture_atlas.textures.len();
            let (num_cells_in_animation, start_index) = match character_option {
                None => {
                    // No character.  Just use all the cells.
                    (u32::try_from(total_num_cells).expect("num cells didn't fit in u32"), 0)
                }
                Some(character) => {
                    // This animated sprite is a character.
                    let row = match character.direction {
                        Direction::North => 3,
                        Direction::South => 0,
                        Direction::East => 2,
                        Direction::West => 1,
                    };
                    let num_cells_per_row = 8;

                    match character.state {
                        CharacterState::Idle    => (1, row * num_cells_per_row + 1),
                        CharacterState::Walking => (3, row * num_cells_per_row),
                    }
                }
            };
            let index_in_animation = (animated_sprite.animation_index + 1) % num_cells_in_animation;
            animated_sprite.animation_index = index_in_animation;
            sprite.index = ((start_index + index_in_animation as usize) % total_num_cells) as u32;
            animated_sprite.done_painting();
        }
    }
}

fn position_display_system(
    mut character_query: Query<(&Transform, &Player)>,
    mut text_query: Query<(&mut Text, &PlayerPositionDisplay)>,
) {
    for (char_transform, player) in character_query.iter_mut() {
        for (mut text, ppd) in text_query.iter_mut() {
            if ppd.player_id == player.id {
                text.value = format!("P{} Position: ({:.1}, {:.1}, {:.1})",
                    player.id + 1,
                    char_transform.translation.x,
                    char_transform.translation.y,
                    char_transform.translation.z);
            }
        }
    }
}
