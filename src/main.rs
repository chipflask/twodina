use std;
use std::convert::TryFrom;

use bevy::{prelude::*, render::camera::{Camera, CameraProjection, OrthographicProjection}};
use bevy_tiled_prototype::{TiledMapCenter, TiledMapComponents, TiledMapPlugin};
use bevy::math::Vec3Swizzles;

mod character;
mod collider;
mod input;
mod items;

use character::{AnimatedSprite, Character, CharacterState, Direction, VELOCITY_EPSILON};
use collider::{Collider, ColliderType, Collision};
use input::{Action, Flag, InputActionSet};
use items::PickUpEvent;

const NUM_PLAYERS: u32 = 2;

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
struct TransientState {
    debug_mode: bool,
}

struct Player {
    id: u32,
}

struct PlayerPositionDisplay {
    player_id: u32,
}

// We have multiple cameras, so this one marks the camera that follows the
// player.
struct PlayerCamera;

// Debug entities will be marked with this so that we can despawn them all when
// debug mode is turned off.
#[derive(Debug, Default)]
struct Debuggable;

const MAP_SKEW: f32 = 1.0; // We liked ~1.4, but this should be done with the camera

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(TiledMapPlugin)
        .add_plugin(input::InputActionPlugin::default())
        .add_plugin(items::ItemsPlugin::default())
        .add_resource(TransientState { debug_mode: false })
        .add_startup_system(setup_system.system())
        .add_system_to_stage(stage::PRE_UPDATE, handle_input_system.system())
        .add_system(animate_sprite_system.system())
        .add_system(move_sprite_system.system())
        .add_system(update_camera_system.system())
        .add_system(position_display_system.system())
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .run();
}


fn handle_input_system(
    input_actions: Res<InputActionSet>,
    mut transient_state: ResMut<TransientState>,
    mut query: Query<(&mut Character, &Player)>,
    mut debuggable: Query<&mut Visible,  With<Debuggable>>
) {

    // check for debug status flag differing from transient_state to determine when to hide/show debug stuff
    if input_actions.has_flag(Flag::Debug) {
        if !transient_state.debug_mode {
            // for now hide, but ideally we spawn debug things here
            for mut visible in debuggable.iter_mut() {
                visible.is_visible = true;
            }
            transient_state.debug_mode = true;
        }
    } else if transient_state.debug_mode {
        // for now show
        for mut visible in debuggable.iter_mut() {
            visible.is_visible = false;
        }
        transient_state.debug_mode = false;
    }


    for (mut character, player) in query.iter_mut() {
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

        if input_actions.is_active(Action::Run, player.id) {
            character.movement_speed = character::RUN_SPEED;
            new_state = match new_state {
                CharacterState::Walking => CharacterState::Running,
                CharacterState::Idle | CharacterState::Running => new_state,
            }
        } else {
            character.movement_speed = character::WALK_SPEED;
        }

        if let Some(direction) = new_direction {
            character.direction = direction;
        }
        character.velocity.x = new_velocity.x;
        character.velocity.y = new_velocity.y;
        // Don't modify z if the character has a z velocity for some reason.

        character.set_state(new_state);
    }
}

const PLAYER_WIDTH: f32 = 31.0;
const PLAYER_HEIGHT: f32 = 32.0;

fn setup_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    transient_state: Res<TransientState>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Cameras.
    commands
        .spawn(Camera2dBundle::default())
        .with(PlayerCamera {})
        .spawn(CameraUiBundle::default());

    // Watch for asset changes.
    asset_server.watch_for_changes().expect("watch for changes");

    // Players.
    for i in 0..NUM_PLAYERS {
        let texture_handle = asset_server.load(format!("sprites/character{}.png", i + 1).as_str());
        let texture_atlas = TextureAtlas::from_grid(texture_handle,
                                                    Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT), 8, 16);
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        let scale = Vec3::splat(4.0);
        let collider_size = Vec2::new(13.0, 4.5);
        let collider_offset = Vec2::new(0.0, -12.5);
        commands
            .spawn(SpriteSheetBundle {
                texture_atlas: texture_atlas_handle,
                transform: Transform::from_scale(scale)
                            .mul_transform(Transform::from_translation(Vec3::new(PLAYER_WIDTH * i as f32 + 20.0, 0.0, 0.0))),
                ..Default::default()
            })
            .with(AnimatedSprite::with_frame_seconds(0.1))
            .with(Character::default())
            .with(Player { id: i })
            .with(Collider::new(ColliderType::Solid, collider_size * scale.xy(), collider_offset * scale.xy()))
            .with_children(|parent| {
                // add a shadow sprite -- is there a more efficient way where we load this just once??
                let shadow_handle = asset_server.load("sprites/shadow.png");
                parent.spawn(SpriteBundle {
                    transform: Transform {
                        translation: Vec3::new(0.0, -13.0, -0.01),
                        scale: Vec3::splat(0.7),
                        ..Default::default()
                    },
                    material: materials.add(shadow_handle.into()),
                    ..Default::default()
                });
                parent.spawn(SpriteBundle {
                    material: materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into()),
                    // Don't scale here since the whole character will be scaled.
                    sprite: Sprite::new(collider_size),
                    transform: Transform::from_translation(Vec3::new(collider_offset.x, collider_offset.y, 0.0)),
                    visible: Visible {
                        is_transparent: true,
                        is_visible: transient_state.debug_mode,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(Debuggable::default());
            })
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
                visible: Visible {
                    is_transparent: true,
                    is_visible: false,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with(PlayerPositionDisplay { player_id: i})
            .with(Debuggable::default());
    }
    // Map
    commands
        .spawn(TiledMapComponents {
            map_asset: asset_server.load("tile_maps/path_map.tmx"),
            center: TiledMapCenter(true),
            origin: Transform {
                translation: Vec3::new(0.0, 0.0, -1000.0),
                scale: Vec3::new(2.0, 2.0 / MAP_SKEW, 1.0),
                ..Default::default()
            },
            ..Default::default()
        });

    // Items
    {
        let texture_handle = asset_server.load("sprites/items.png");
        let items = vec![
            // Shield.
            bevy::sprite::Rect {
                min: Vec2::new(194.0, 18.0),
                max: Vec2::new(206.0, 31.0),
            }
        ];
        // Shield.
        let scale = Vec3::splat(3.0);
        let unequipped_transform = Transform::from_scale(scale);
        let mut equipped_transform = unequipped_transform.clone();
        equipped_transform.translation = Vec3::new(0.0, -10.0, 0.0);

        let texture_atlas = TextureAtlas {
            texture: texture_handle,
            size: Vec2::new(432.0, 176.0),
            textures: items,
            texture_handles: None,
        };
        let texture_atlas_handle = texture_atlases.add(texture_atlas);

        let collider_size = Vec2::new(12.0, 13.0);
        let collider_offset = Vec2::new(0.0, 0.0);

        for x_position in vec![-50.0, 140.0] {
            commands
                .spawn(SpriteSheetBundle {
                    texture_atlas: texture_atlas_handle.clone(),
                    transform: unequipped_transform.mul_transform(
                        Transform::from_translation(Vec3::new(x_position, 0.0, 0.0))),
                    ..Default::default()
                })
                .with(Collider::new(ColliderType::PickUp, collider_size * scale.xy(), collider_offset * scale.xy()))
                .with(items::EquippedTransform { transform: equipped_transform })
                .with_children(|parent| {
                    // Add a shadow sprite.
                    let shadow_handle = asset_server.load("sprites/shadow.png");
                    parent.spawn(SpriteBundle {
                        transform: Transform {
                            translation: Vec3::new(0.0, -5.0, -0.01),
                            scale: Vec3::splat(0.5),
                            ..Default::default()
                        },
                        material: materials.add(shadow_handle.into()),
                        ..Default::default()
                    });
                    parent
                        .spawn(SpriteBundle {
                            material: materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into()),
                            // Don't scale here since the whole character will be scaled.
                            sprite: Sprite::new(collider_size),
                            transform: Transform::from_translation(Vec3::new(collider_offset.x, collider_offset.y, 0.0)),
                            visible: Visible {
                                is_transparent: true,
                                is_visible: transient_state.debug_mode,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with(Debuggable::default());
                });
        }
    }
}

fn move_sprite_system(
    time: Res<Time>,
    mut pick_up_events: ResMut<Events<PickUpEvent>>,
    mut char_query: Query<(Entity, &mut Character, &mut Transform, &GlobalTransform, &Collider)>,
    mut collider_query: Query<(Entity, &Collider, &GlobalTransform)>,
) {
    for (char_entity, mut character, mut transform, char_global, char_collider) in char_query.iter_mut() {
        if character.velocity.abs_diff_eq(Vec2::zero(), VELOCITY_EPSILON) {
            // Character has zero velocity.  Nothing to do.
            continue;
        }
        let mut delta: Vec2 = character.velocity * time.delta_seconds() * character.movement_speed;
        delta.y /= MAP_SKEW;
        // should stay between +- 2000.0

        let char_aabb = char_collider.bounding_volume_with_translation(char_global, delta);

        let mut char_collision = Collision::NoCollision;
        for (collider_entity, collider, collider_global) in collider_query.iter_mut() {
            // Shouldn't collide with itself.
            if std::ptr::eq(char_collider, collider) {
                continue;
            }
            let collision = collider.intersect(collider_global, &char_aabb);
            match collision {
                Collision::Solid => {
                    char_collision = collision;
                    break;
                }
                Collision::PickUp => {
                    pick_up_events.send(PickUpEvent::new(
                        char_entity,
                        collider_entity,
                    ));

                    // Upgrade NoCollision; don't downgrade Solid.
                    match char_collision {
                        Collision::NoCollision => {
                            char_collision = collision;
                        }
                        Collision::Solid | Collision::PickUp => (),
                    }
                }
                Collision::NoCollision => (),
            }
        }
        if !char_collision.is_solid() {
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
            transform.translation.z = -transform.translation.y / 100.0;
        }
        character.collision = char_collision;
    }
}

fn bounding_box(translation: Vec3, size: Vec2) -> Rect<f32> {
    let half_width = size.x / 2.0;
    let half_height = size.y / 2.0;
    Rect {
        left: translation.x - half_width,
        right: translation.x + half_width,
        top: translation.y + half_height,
        bottom: translation.y - half_height,
    }
}

// Returns the bounding box that includes both the given bounding boxes.
fn expand_bounding_box(r1: &Rect<f32>, r2: &Rect<f32>) -> Rect<f32> {
    Rect {
        left: r1.left.min(r2.left),
        right: r1.right.max(r2.right),
        top: r1.top.max(r2.top),
        bottom: r1.bottom.min(r2.bottom),
    }
}

fn rect_center(r: &Rect<f32>) -> Vec2 {
    // Don't overflow.
    Vec2::new(r.left + 0.5 * (r.right - r.left), r.bottom + 0.5 * (r.top - r.bottom))
}

#[allow(dead_code)]
fn rect_half_width_height(r: &Rect<f32>) -> Vec2 {
    Vec2::new(0.5 * (r.right - r.left), 0.5 * (r.top - r.bottom))
}

fn rect_width_height(r: &Rect<f32>) -> Vec2 {
    Vec2::new(r.right - r.left, r.top - r.bottom)
}

fn rect_expand_by(r: &Rect<f32>, amount: f32) -> Rect<f32> {
    Rect {
        left: r.left - amount,
        right: r.right + amount,
        top: r.top + amount,
        bottom: r.bottom - amount,
    }
}

// wh is width and height.
// aspect_ratio is the desired width / height.
fn expanded_to_aspect_ratio(wh: &Vec2, aspect_ratio: f32) -> Vec2 {
    let h_based_on_w = wh.x / aspect_ratio;
    if h_based_on_w > wh.y {
        Vec2::new(wh.x, h_based_on_w)
    } else {
        let w_based_on_h = wh.y * aspect_ratio;

        Vec2::new(w_based_on_h, wh.y)
    }
}

fn viewport(camera_transform: &GlobalTransform, projection: &OrthographicProjection) -> Rect<f32> {
    let translation = &camera_transform.translation;
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
    windows: Res<Windows>,
    mut player_query: Query<&GlobalTransform, With<Player>>,
    mut camera_query: Query<(&mut Transform,
                            &GlobalTransform,
                            &mut OrthographicProjection,
                            &mut Camera),
                            With<PlayerCamera>>,
) {
    // Amount of margin between edge of view and character.
    let margin = 100.0;

    // Get bounding box of all players.
    let mut full_bb = None;
    let mut num_players = 0;
    let mut player_translation = Vec3::zero();
    for player_transform in player_query.iter_mut() {
        num_players += 1;
        // Is sprite in view frame?
        // println!("player translation {:?}", player_transform.translation);
        let char_translation = player_transform.translation;
        let char_size = Vec2::new(PLAYER_WIDTH * player_transform.scale.x, PLAYER_HEIGHT * player_transform.scale.y);
        let char_rect = bounding_box(char_translation, char_size);
        // println!("char_rect {:?}", char_rect);
        full_bb = match full_bb {
            None => {
                player_translation = player_transform.translation;
                Some(char_rect)
            }
            Some(bb) => Some(expand_bounding_box(&bb, &char_rect)),
        };
    }

    if let Some(full_bb) = full_bb {
        let window = windows.get_primary().expect("should be at least one window so we can compute aspect ratio");
        let win_width = window.width();
        let win_height = window.height();
        let aspect_ratio = win_width / win_height;

        // Add margin.
        // TODO: Handle case when window is smaller than margin.
        let full_bb = rect_expand_by(&full_bb, margin);

        for (mut camera_transform, camera_global, mut projection, mut camera) in camera_query.iter_mut() {
            // TODO: this only needs to happen once, so maybe there is a better place to do this?
            projection.near = -2000.0;
            projection.far = 2000.0;
            // println!("projection {:?}", projection);
            // println!("camera_transform {:?}", camera_transform);
            // println!("camera_global {:?}", camera_global);
            // Note: We don't support camera rotation or scale.
            let camera_rect = viewport(&camera_global, &projection);
            // println!("camera_rect {:?}", camera_rect);
            if num_players <= 1 {
                // Center on the player if not in view.
                let is_player_in_view = is_rect_completely_inside(&full_bb, &camera_rect);
                if !is_player_in_view {
                    // Mutate the transform, never the global transform.
                    camera_transform.translation = player_translation;
                }
            } else {
                // Center on the center of the bounding box of all players.
                let c = rect_center(&full_bb);
                camera_transform.translation.x = c.x;
                camera_transform.translation.y = c.y;

                // Zoom so that all players are in view.
                let mut wh = rect_width_height(&full_bb);
                wh = expanded_to_aspect_ratio(&wh, aspect_ratio);
                // Never zoom in smaller than the window.
                if wh.x < win_width || wh.y < win_height {
                    wh = Vec2::new(win_width, win_height);
                }
                projection.update(wh.x, wh.y);
                camera.projection_matrix = projection.get_projection_matrix();
                camera.depth_calculation = projection.depth_calculation();
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
        // If character just started walking or is colliding, always show
        // stepping frame, and do it immediately.  Don't wait for the timer's
        // next tick.
        let is_stepping = character_option.map_or(false, |ch| {
            let state = ch.state();

            ch.is_stepping() && (ch.collision.is_solid() || state != ch.previous_state())
        });

        // Reset to the beginning of the animation when the character becomes
        // idle.
        if let Some(character) = character_option {
            if character.did_just_become_idle() {
                animated_sprite.reset();
            }
        }

        animated_sprite.timer.tick(time.delta_seconds());
        if is_stepping || animated_sprite.timer.finished() {
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

                    match character.state() {
                        CharacterState::Idle    => (1, row * num_cells_per_row + 1),
                        CharacterState::Walking => (4, row * num_cells_per_row),
                        CharacterState::Running => (4, (row + 4) * num_cells_per_row),
                    }
                }
            };
            let mut new_anim_index = if is_stepping {
                // Index of taking a step.
                2
            } else {
                animated_sprite.animation_index + 1
            };
            new_anim_index = new_anim_index % num_cells_in_animation;
            animated_sprite.animation_index = new_anim_index;
            sprite.index = ((start_index + new_anim_index as usize) % total_num_cells) as u32;
        }
    }
}

fn position_display_system(
    mut character_query: Query<(&Transform, &Player, &Character)>,
    mut text_query: Query<(&mut Text, &PlayerPositionDisplay)>,
) {
    for (char_transform, player, character) in character_query.iter_mut() {
        for (mut text, ppd) in text_query.iter_mut() {
            if ppd.player_id == player.id {
                text.value = format!("P{} Position: ({:.1}, {:.1}, {:.1}) collision={:?}",
                    player.id + 1,
                    char_transform.translation.x,
                    char_transform.translation.y,
                    char_transform.translation.z,
                    character.collision);
            }
        }
    }
}
