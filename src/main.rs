use std;
use std::convert::TryFrom;

use bevy::{asset::{Asset, HandleId}, prelude::*, render::camera::{Camera, CameraProjection, OrthographicProjection}, utils::HashSet};
use bevy_tiled_prototype::{DebugConfig, Map, Object, ObjectReadyEvent, ObjectShape, TiledMapCenter, TiledMapComponents, TiledMapPlugin};
use bevy::math::Vec3Swizzles;

mod character;
mod collider;
mod dialogue;
mod input;
mod items;

use character::{AnimatedSprite, Character, CharacterState, Direction, VELOCITY_EPSILON};
use collider::{Collider, ColliderBehavior, Collision};
use dialogue::Dialogue;
use input::{Action, Flag, InputActionSet};
use items::Inventory;
use stage::UPDATE;

const DEBUG_MODE_DEFAULT: bool = false;
const TILED_MAP_SCALE: f32 = 2.0;

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
struct TransientState {
    debug_mode: bool,
    current_map: Handle<Map>,
    current_dialogue: Option<Entity>,
    default_blue: Handle<ColorMaterial>,
    button_color: Handle<ColorMaterial>,
    button_hovered_color: Handle<ColorMaterial>,
    button_pressed_color: Handle<ColorMaterial>,
}

// Tag for the menu system UI.
struct MenuUi;

enum MenuButton {
    OnePlayer,
    TwoPlayers,
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

#[derive(Debug, Copy, Clone)]
pub enum AppState {
    Loading,
    Menu,
    InGame,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Loading
    }
}

#[derive(Debug, Default)]
struct LoadProgress {
    handles: HashSet<HandleUntyped>,
    next_state: AppState,
    // progress: f32,
}

impl LoadProgress {
    pub fn add<T: Asset>(&mut self, handle: Handle<T>) -> Handle<T> {
        self.handles.insert(handle.clone_untyped());

        handle
    }

    pub fn reset(&mut self) {
        self.handles.clear();
    }
}

// run loop stages
pub const EARLY: &str = "EARLY";
pub const LATER: &str = "LATER";

fn main() {
    App::build()
        .add_resource(State::new(AppState::default()))
        .add_resource(LoadProgress::default())
        // add stages to run loop
        .add_stage_after(UPDATE, EARLY, StateStage::<AppState>::default())
        .add_stage_after(EARLY, LATER, StateStage::<AppState>::default())
        // add plugins
        .add_plugins(DefaultPlugins)
        .add_plugin(TiledMapPlugin)
        .add_plugin(dialogue::DialoguePlugin::default())
        .add_plugin(input::InputActionPlugin::default())
        .add_plugin(items::ItemsPlugin::default())
        // init
        .add_startup_system(setup_system.system())
        // loading
        .on_state_update(LATER, AppState::Loading, wait_for_asset_loading_system.system())
        //
        // menu
        .on_state_enter(EARLY, AppState::Menu, setup_menu_system.system())
        .on_state_update(LATER, AppState::Menu, menu_system.system().chain(setup_players_system.system()))
        .on_state_update(LATER, AppState::Menu, bevy::input::system::exit_on_esc_system.system())
        .on_state_update(LATER, AppState::Menu, map_item_system.system())
        .on_state_exit(EARLY, AppState::Menu, cleanup_menu_system.system())

        // in-game:
        .on_state_update(EARLY, AppState::InGame, handle_input_system.system())
        .on_state_update(LATER, AppState::InGame, animate_sprite_system.system())
        .on_state_update(LATER, AppState::InGame, move_character_system.system())
        .on_state_update(LATER, AppState::InGame, update_camera_system.system())
        .on_state_update(LATER, AppState::InGame, position_display_system.system())
        .on_state_update(LATER, AppState::InGame, map_item_system.system())
        .on_state_update(LATER, AppState::InGame, bevy::input::system::exit_on_esc_system.system())
        .run();
}

fn wait_for_asset_loading_system(
    mut state: ResMut<State<AppState>>,
    mut load_progress: ResMut<LoadProgress>,
    asset_server: Res<AssetServer>,
) {
    let handle_ids = load_progress.handles.iter()
        .map(|handle| HandleId::from(handle));
    match asset_server.get_group_load_state(handle_ids) {
        bevy::asset::LoadState::NotLoaded => {}
        bevy::asset::LoadState::Loading => {}
        bevy::asset::LoadState::Loaded => {
            state.set_next(load_progress.next_state).expect("couldn't change state when assets finished loading");
            load_progress.reset();
        }
        // TODO: Handle failed loading of assets.
        bevy::asset::LoadState::Failed => {}
    }
}


fn handle_input_system(
    input_actions: Res<InputActionSet>,
    mut transient_state: ResMut<TransientState>,
    mut query: Query<(&mut Character, &Player)>,
    mut debuggable: Query<&mut Visible, With<Debuggable>>,
    mut dialogue_query: Query<&mut Dialogue>,
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

        if let Some(entity) = transient_state.current_dialogue {
            if input_actions.is_active(Action::Accept, player.id) {
                let mut dialogue = dialogue_query.get_mut(entity).expect("Couldn't find current dialogue entity");
                dialogue.advance();
            }
        }
    }
}

fn setup_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut to_load: ResMut<LoadProgress>,
) {
    // Default materials
    let default_blue = materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into());
    // Cameras.
    commands
        .spawn(Camera2dBundle {
            orthographic_projection: OrthographicProjection {
                near: -2000.0,
                far: 2000.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .with(PlayerCamera {})
        .spawn(CameraUiBundle::default());

    // Watch for asset changes.
    asset_server.watch_for_changes().expect("watch for changes");

    // Map - has some objects
    // transient_state: Res<TransientState>,
    let transient_state = TransientState {
        debug_mode: DEBUG_MODE_DEFAULT,
        current_map: to_load.add(asset_server.load("maps/melle/sandyrocks.tmx")),
        current_dialogue: None,
        default_blue: default_blue.clone(),
        button_color: materials.add(Color::rgb(0.4, 0.4, 0.9).into()),
        button_hovered_color: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
        button_pressed_color: materials.add(Color::rgb(0.3, 0.3, 0.8).into()),
    };
    commands
        .spawn(TiledMapComponents {
            map_asset: transient_state.current_map.clone(),
            center: TiledMapCenter(true),
            origin: Transform {
                translation: Vec3::new(0.0, 0.0, -100.0),
                scale: Vec3::new(TILED_MAP_SCALE, TILED_MAP_SCALE / MAP_SKEW, 1.0),
                ..Default::default()
            },
            debug_config: DebugConfig {
                enabled: DEBUG_MODE_DEFAULT,
                material: Some(default_blue.clone()),
            },
            ..Default::default()
        });

    commands.insert_resource(transient_state);

    to_load.next_state = AppState::Menu;
}

fn setup_menu_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    transient_state: Res<TransientState>,
) {
    commands
        // Root
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                flex_direction: FlexDirection::ColumnReverse,
                // Horizontally center child text
                justify_content: JustifyContent::Center,
                // Vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with(MenuUi {})
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle {
                style: Style {
                    margin: Rect::all(Val::Px(5.0)),
                    ..Default::default()
                },
                text: Text {
                    value: "Celebration 2021".to_string(),
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    style: TextStyle {
                        font_size: 60.0,
                        color: Color::BLACK,
                        ..Default::default()
                    },
                },
                ..Default::default()
            });

            // Start button 1 player.
            parent.spawn(ButtonBundle {
                style: Style {
                    size: Size::new(Val::Px(170.0), Val::Px(65.0)),
                    margin: Rect::all(Val::Px(5.0)),
                    // Horizontally center child text
                    justify_content: JustifyContent::Center,
                    // Vertically center child text
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: transient_state.button_color.clone(),
                ..Default::default()
            })
            .with(MenuButton::OnePlayer)
            .with_children(|parent| {
                parent.spawn(TextBundle {
                    text: Text {
                        value: "1 Player".to_string(),
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        style: TextStyle {
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..Default::default()
                        },
                    },
                    ..Default::default()
                });
            });

            // Start button 2 players.
            parent.spawn(ButtonBundle {
                style: Style {
                    size: Size::new(Val::Px(170.0), Val::Px(65.0)),
                    margin: Rect::all(Val::Px(5.0)),
                    // Horizontally center child text
                    justify_content: JustifyContent::Center,
                    // Vertically center child text
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: transient_state.button_color.clone(),
                ..Default::default()
            })
            .with(MenuButton::TwoPlayers)
            .with_children(|parent| {
                parent.spawn(TextBundle {
                    text: Text {
                        value: "2 Players".to_string(),
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        style: TextStyle {
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..Default::default()
                        },
                    },
                    ..Default::default()
                });
            });
        });
}

fn cleanup_menu_system(
    commands: &mut Commands,
    query: Query<Entity, With<MenuUi>>,
) {
    for entity in query.iter() {
        commands.despawn_recursive(entity);
    }
}

enum MenuAction {
    Nil,
    LoadPlayers { num_players: u8 },
}

fn menu_system(
    transient_state: ResMut<TransientState>,
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &MenuButton),
        (Mutated<Interaction>, With<Button>),
    >,
) -> MenuAction {
    let mut action = MenuAction::Nil;
    for (interaction, mut material, button_choice) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                match button_choice {
                    MenuButton::OnePlayer => {
                        action = MenuAction::LoadPlayers { num_players: 1 };
                    }
                    MenuButton::TwoPlayers => {
                        action = MenuAction::LoadPlayers { num_players: 2 };
                    }
                }
            }
            Interaction::Hovered => {
                *material = transient_state.button_hovered_color.clone();
            }
            Interaction::None => {
                *material = transient_state.button_pressed_color.clone();
            }
        }
    }

    action
}

const PLAYER_WIDTH: f32 = 31.0;
const PLAYER_HEIGHT: f32 = 32.0;

fn setup_players_system(
    In(menu_action): In<MenuAction>,
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut transient_state: ResMut<TransientState>,
    mut state: ResMut<State<AppState>>,
    mut to_load: ResMut<LoadProgress>,
) {
    let num_players = match menu_action {
        MenuAction::Nil => return,
        MenuAction::LoadPlayers { num_players } => num_players,
    };

    state.set_next(AppState::Loading).expect("Set Next failed");
    to_load.next_state = AppState::InGame;

    // Load dialog.
    let level_dialogue = to_load.add(asset_server.load("dialogue/level1.dialogue"));
    commands
        .spawn(TextBundle {
            text: Text {
                font: to_load.add(asset_server.load("fonts/FiraSans-Bold.ttf")),
                value: "".to_string(),
                style: TextStyle {
                    color: Color::rgb(0.2, 0.2, 0.2),
                    font_size: 24.0,
                    ..Default::default()
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    left: Val::Px(20.0),
                    right: Val::Px(20.0),
                    bottom: Val::Px(20.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with(Dialogue {
            handle: level_dialogue,
            ..Default::default()
        })
        .current_entity()
        .map(|entity| transient_state.current_dialogue = Some(entity));

    let default_red = materials.add(Color::rgba(1.0, 0.4, 0.9, 0.8).into());
    // Players.
    for i in 0..num_players {
        let texture_handle = to_load.add(asset_server.load(format!("sprites/character{}.png", i + 1).as_str()));
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT),
            8,
            16,
        );
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        let scale = Vec3::splat(4.0);
        let collider_size = Vec2::new(13.0, 4.5);
        let collider_offset = Vec2::new(0.0, -12.5);
        // This should match the move_character_system.
        let initial_z = z_from_y(collider_offset.y);
        commands
            .spawn(SpriteSheetBundle {
                texture_atlas: texture_atlas_handle,
                transform: Transform::from_scale(scale)
                    .mul_transform(Transform::from_translation(
                        Vec3::new(PLAYER_WIDTH * i as f32 + 20.0, 0.0, initial_z))),
                ..Default::default()
            })
            .with(AnimatedSprite::with_frame_seconds(0.1))
            .with(Character::default())
            .with(Player { id: u32::from(i) })
            .with(items::Inventory::default())
            .with(Collider::new(
                ColliderBehavior::Obstruct,
                collider_size * scale.xy(),
                collider_offset * scale.xy(),
            ))
            .with_children(|parent| {
                // add a shadow sprite -- is there a more efficient way where we load this just once??
                let shadow_handle = to_load.add(asset_server.load("sprites/shadow.png"));
                parent.spawn(SpriteBundle {
                    transform: Transform {
                        translation: Vec3::new(0.0, -13.0, -0.01),
                        scale: Vec3::splat(0.7),
                        ..Default::default()
                    },
                    material: materials.add(shadow_handle.into()),
                    ..Default::default()
                });
                // collider debug indicator - TODO: refactor into Collider::new_with_debug(parent, collider_size, scale)
                parent.spawn(SpriteBundle {
                    material: transient_state.default_blue.clone(),
                    // Don't scale here since the whole character will be scaled.
                    sprite: Sprite::new(collider_size),
                    transform: Transform::from_translation(Vec3::new(collider_offset.x, collider_offset.y, 0.0)),
                    visible: Visible {
                        is_transparent: true,
                        is_visible: DEBUG_MODE_DEFAULT,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(Debuggable::default());
                // Center debug indicator.
                parent.spawn(SpriteBundle {
                    material: default_red.clone(),
                    // Don't scale here since the whole character will be scaled.
                    sprite: Sprite::new(Vec2::new(5.0, 5.0)),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
                    visible: Visible {
                        is_transparent: true,
                        is_visible: DEBUG_MODE_DEFAULT,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(Debuggable::default());
            })
            .spawn(TextBundle {
                text: Text {
                    font: to_load.add(asset_server.load("fonts/FiraSans-Bold.ttf")),
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
            .with(PlayerPositionDisplay { player_id: u32::from(i) })
            .with(Debuggable::default());
    }

    // Items
    {
        let texture_handle = to_load.add(asset_server.load("sprites/items.png"));
        let items = vec![
            // Shield.
            bevy::sprite::Rect {
                min: Vec2::new(194.0, 18.0),
                max: Vec2::new(206.0, 31.0),
            },
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
                        Transform::from_translation(Vec3::new(x_position, 0.0, z_from_y(0.0)))),
                    ..Default::default()
                })
                .with(Collider::new(ColliderBehavior::PickUp, collider_size * scale.xy(), collider_offset * scale.xy()))
                .with(items::EquippedTransform { transform: equipped_transform })
                .with_children(|parent| {
                    // Add a shadow sprite.
                    let shadow_handle = to_load.add(asset_server.load("sprites/shadow.png"));
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
                            material: transient_state.default_blue.clone(),
                            // Don't scale here since the whole character will be scaled.
                            sprite: Sprite::new(collider_size),
                            transform: Transform::from_translation(Vec3::new(
                                collider_offset.x,
                                collider_offset.y,
                                0.0,
                            )),
                            visible: Visible {
                                is_transparent: true,
                                is_visible: DEBUG_MODE_DEFAULT,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with(Debuggable::default());
                });
        }
    }
}

// Return the Z translation for a given Y translation.  Z determines occlusion.
#[inline]
fn z_from_y(y: f32) -> f32 {
    -y / 100.0
}

fn move_character_system(
    time: Res<Time>,
    mut interaction_event: ResMut<Events<items::Interaction>>,
    mut char_query: Query<(Entity, &mut Character, &mut Transform, &GlobalTransform)>,
    mut collider_query: Query<(Entity, &mut Collider, &GlobalTransform)>,
) {
    let mut interaction_colliders: HashSet<Entity> = Default::default();
    for (char_entity, mut character, mut transform, char_global) in char_query.iter_mut() {
        let char_collider = collider_query.get_component::<Collider>(char_entity).unwrap().clone();
        if character.velocity.abs_diff_eq(Vec2::zero(), VELOCITY_EPSILON) {
            // Character has zero velocity.  Nothing to do.
            continue;
        }
        let mut delta: Vec2 = character.velocity * time.delta_seconds() * character.movement_speed;
        delta.y /= MAP_SKEW;
        // should stay between +- 2000.0

        let char_aabb = char_collider.bounding_volume_with_translation(char_global, delta);

        let mut char_collision = Collision::Nil;
        for (collider_entity, collider, collider_global) in collider_query.iter_mut() {
            // Shouldn't collide with itself.
            if collider_entity == char_entity {
                continue;
            }
            let collision = collider.intersect(collider_global, &char_aabb);
            match collision {
                Collision::Obstruction => {
                    char_collision = collision;
                    break;
                }
                Collision::Interaction(behavior) => {
                    interaction_colliders.insert(collider_entity);
                    interaction_event.send(items::Interaction::new(
                        char_entity,
                        collider_entity,
                        behavior,
                    ));

                    // Upgrade Collision::Nil; don't downgrade Obstruction.
                    match char_collision {
                        Collision::Nil => {
                            char_collision = collision;
                        }
                        Collision::Obstruction | Collision::Interaction(_) => (),
                    }
                }
                Collision::Nil => (),
            }
        }
        if !char_collision.is_solid() {
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
            // Z needs to reflect where the character is on the ground, and
            // presumably, that's where the character collides.  So we add the
            // collider's Z offset to the translation.
            transform.translation.z = z_from_y(transform.translation.y + char_collider.offset.y);
        }
        character.collision = char_collision;
    }
    for entity in interaction_colliders.iter() {
        if let Ok(mut collider) = collider_query.get_component_mut::<Collider>(*entity) {
            collider.behavior = ColliderBehavior::Ignore;
        }
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
    let margin_1p = 75.0;
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
        let margin_amount = if num_players <= 1 { margin_1p } else { margin };
        // Add margin.
        // TODO: Handle case when window is smaller than margin.
        let full_bb = rect_expand_by(&full_bb, margin_amount);
        let margin_vec =  Vec3::new(
            (win_width - margin_amount * 1.1) / win_width,
            (win_height - margin_amount * 1.1) / win_height, 1.0);

        for (mut camera_transform, camera_global, mut projection, mut camera) in camera_query.iter_mut() {
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
                    let mut new_cam_translation = camera_transform.translation.clone();
                    let mut v1 = camera_transform.translation.clone() - player_translation;

                    v1.x = margin_vec.x.min((v1.x.abs() / win_width).abs()) * v1.x.signum() * win_width;
                    v1.y = margin_vec.y.min((v1.y.abs() / win_height).abs()) * v1.y.signum() * win_height;
                    // println!("{:?} - {:?}", v1, margin_vec);

                    new_cam_translation = new_cam_translation - v1 * 2.0;
                    new_cam_translation.z = camera_transform.translation.z;
                    camera_transform.translation = new_cam_translation;

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

fn map_item_system(
    commands: &mut Commands,
    new_item_query: Query<&Object>,
    transient_state: Res<TransientState>,
    mut event_reader: Local<EventReader<ObjectReadyEvent>>,
    map_ready_events: Res<Events<ObjectReadyEvent>>,
    // maps: Res<Assets<Map>>,
) {
    for event in event_reader.iter(&map_ready_events) {
        if transient_state.current_map != event.map_handle {
            continue;
        }
        // let map = maps.get(event.map_handle).expect("Expected to find map from ObjectReadyEvent");
        if let Ok(object) = new_item_query.get(event.entity) {
            let collider_size = TILED_MAP_SCALE * match object.shape {
                ObjectShape::Rect { width, height } | ObjectShape::Ellipse { width, height } =>
                    Vec2::new(width, height),
                ObjectShape::Polyline { points: _ } | ObjectShape::Polygon { points: _ } | ObjectShape::Point(_, _) =>
                    Vec2::new(40.0, 40.0),
            };

            // we should have actual types based on object name
            // and add components based on that
            let collider_type = match object.name.as_ref() {
                "biggem" => {
                    if !object.visible {
                        ColliderBehavior::Ignore
                    } else {
                        ColliderBehavior::Collect
                    }
                },
                "gem" => {
                    ColliderBehavior::Collect
                }
                _ => {
                    if object.is_shape() { // allow hide/show objects without images
                        commands.insert_one(event.entity, Debuggable::default());
                    }
                    ColliderBehavior::Obstruct
                }
            };

            let collider_component = Collider::new(collider_type, collider_size, Vec2::new(0.0, 0.0));
            commands.insert_one(event.entity, collider_component);
        }
    }
}

fn position_display_system(
    mut character_query: Query<(&Transform, &Player, &Character, &Inventory)>,
    mut text_query: Query<(&mut Text, &PlayerPositionDisplay)>,
) {
    for (char_transform, player, character, inventory) in character_query.iter_mut() {
        for (mut text, ppd) in text_query.iter_mut() {
            if ppd.player_id == player.id {
                text.value = format!(
                    "P{} Position: ({:.1}, {:.1}, {:.1}) collision={:?} gems={:?}",
                    player.id + 1,
                    char_transform.translation.x,
                    char_transform.translation.y,
                    char_transform.translation.z,
                    character.collision,
                    inventory.num_gems
                );
            }
        }
    }
}
