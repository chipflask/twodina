use std;
use std::convert::TryFrom;

use bevy::{asset::{Asset, HandleId}, prelude::*, render::camera::{Camera, CameraProjection, OrthographicProjection}, utils::{HashMap, HashSet}};
use bevy_tiled_prototype::{DebugConfig, Map, Object, ObjectReadyEvent, ObjectShape, TiledMapCenter, TiledMapComponents, TiledMapPlugin};
use bevy::math::Vec3Swizzles;

mod character;
mod collider;
mod dialogue;
mod input;
mod items;

use character::{AnimatedSprite, Character, CharacterState, Direction, VELOCITY_EPSILON};
use collider::{Collider, ColliderBehavior, Collision};
use dialogue::{Dialogue, DialogueAsset, DialogueEvent, DialoguePlaceholder};
use input::{Action, Flag, InputActionSet};
use items::Inventory;
use stage::UPDATE;

const DEBUG_MODE_DEFAULT: bool = false;
const TILED_MAP_SCALE: f32 = 2.0;

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
pub struct TransientState {
    debug_mode: bool,
    start_dialogue_shown: bool,
    current_dialogue: Option<Entity>,
    current_map: Handle<Map>,
    next_map: Option<Handle<Map>>,
    loaded_maps: HashSet<Handle<Map>>,
    entity_visibility: HashMap<Entity, bool>, // this is a minor memory leak until maps aren't recreated

    default_blue: Handle<ColorMaterial>,
    default_red: Handle<ColorMaterial>,
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

// The UI element that displays dialogue.
struct DialogueWindow;

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
pub struct LoadProgress {
    handles: HashSet<HandleUntyped>,
    next_state: AppState,
    next_dialogue: Option<String>,
    // progress: f32,
}

impl LoadProgress {
    pub fn add<T: Asset>(&mut self, handle: Handle<T>) -> Handle<T> {
        self.handles.insert(handle.clone_untyped());
        handle
    }

    pub fn reset(&mut self) {
        self.handles.clear();
        self.next_dialogue = None;
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
        .on_state_enter(EARLY, AppState::InGame, in_game_start_system.system())
        .on_state_update(EARLY, AppState::InGame, handle_input_system.system())
        .on_state_update(LATER, AppState::InGame, animate_sprite_system.system())
        .on_state_update(LATER, AppState::InGame, move_character_system.system())
        .on_state_update(LATER, AppState::InGame, update_camera_system.system())
        .on_state_update(LATER, AppState::InGame, position_display_system.system())
        .on_state_update(LATER, AppState::InGame, map_item_system.system())
        .on_state_update(LATER, AppState::InGame, display_dialogue_system.system())
        .on_state_update(LATER, AppState::InGame, bevy::input::system::exit_on_esc_system.system())
        .run();
}

fn wait_for_asset_loading_system(
    mut state: ResMut<State<AppState>>,
    mut load_progress: ResMut<LoadProgress>,
    asset_server: Res<AssetServer>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: ResMut<Events<DialogueEvent>>,
) {
    let handle_ids = load_progress.handles.iter()
        .map(|handle| HandleId::from(handle));
    match asset_server.get_group_load_state(handle_ids) {
        bevy::asset::LoadState::NotLoaded => {}
        bevy::asset::LoadState::Loading => {}
        bevy::asset::LoadState::Loaded => {
            state.set_next(load_progress.next_state).expect("couldn't change state when assets finished loading");
            if let Some(node_name) = &load_progress.next_dialogue {
                for mut dialogue in dialogue_query.iter_mut() {
                    dialogue.begin_optional(node_name.as_ref(), &mut dialogue_events);
                }
            }
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
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: ResMut<Events<DialogueEvent>>,
    mut debuggable: Query<(&mut Visible, Option<&Handle<Map>>), With<Debuggable>>,
) {
    // check for debug status flag differing from transient_state to determine when to hide/show debug stuff
    if input_actions.has_flag(Flag::Debug) != transient_state.debug_mode {
        transient_state.debug_mode = !transient_state.debug_mode;
        // for now hide, but ideally we spawn debug things here
        for (mut visible, map_option) in debuggable.iter_mut() {
            let mut in_current_map = true;
            map_option.map(|map_handle| {
                in_current_map = *map_handle == transient_state.current_map;
            });
            visible.is_visible = in_current_map && transient_state.debug_mode;
        }
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
                dialogue.advance(&mut dialogue_events);
            }
        }
    }
}

fn setup_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut to_load: ResMut<LoadProgress>,
    mut query: Query<(Entity, &Handle<Map>, &mut Visible)>,
) {
    // Default materials
    let default_blue = materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into());
    let default_red = materials.add(Color::rgba(1.0, 0.4, 0.9, 0.8).into());
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
    let mut transient_state = TransientState {
        debug_mode: DEBUG_MODE_DEFAULT,
        start_dialogue_shown: false,
        current_map: to_load.add(asset_server.load("maps/sandyrocks.tmx")),
        current_dialogue: None,
        next_map: None,
        loaded_maps: HashSet::default(),
        entity_visibility: HashMap::default(),

        default_blue: default_blue.clone(),
        default_red: default_red.clone(),
        button_color: materials.add(Color::rgb(0.4, 0.4, 0.9).into()),
        button_hovered_color: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
        button_pressed_color: materials.add(Color::rgb(0.3, 0.3, 0.8).into()),
    };
    load_next_map(commands, &mut transient_state, &mut query);
    commands.insert_resource(transient_state);

    to_load.next_state = AppState::Menu;
}

pub fn load_next_map(
    commands: &mut Commands,
    transient_state: &mut TransientState,
    query: &mut Query<(Entity, &Handle<Map>, &mut Visible)>,
) {
    for (entity, map_owner, mut visible) in query.iter_mut() {
        if *map_owner != transient_state.current_map {
            transient_state.entity_visibility.insert(entity.clone(), visible.is_visible);
            commands.remove_one::<Draw>(entity);
            visible.is_visible = false;
        } else {
            let is_visible = transient_state.entity_visibility.get(&entity).unwrap_or(&false);
            // ^ should default object.visible if object
            commands.insert_one(entity, Draw::default());
            visible.is_visible = *is_visible;
        }
    }
    // don't spawn if map already exists
    if transient_state.loaded_maps.contains(&transient_state.current_map) {
        return;
    }
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
                material: Some(transient_state.default_blue.clone()),
            },
            ..Default::default()
        });
    transient_state.loaded_maps.insert(transient_state.current_map.clone());
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

// for 'naked base'
// const PLAYER_WIDTH: f32 = 26.0;
// const PLAYER_HEIGHT: f32 = 36.0;
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

    // Load dialogue.
    let level_dialogue = to_load.add(asset_server.load("dialogue/level1.dialogue"));
    // Root node.
    commands.spawn(NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            flex_direction: FlexDirection::Column,
            // Aligns the dialogue window to the bottom of the window.  Yes, it
            // starts from the bottom!
            justify_content: JustifyContent::FlexStart,
            // Center horizontally.
            align_items: AlignItems::Center,
            ..Default::default()
        },
        material: materials.add(Color::NONE.into()),
        ..Default::default()
    })
    .with_children(|parent| {
        // Dialogue window.
        parent.spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(95.0), Val::Px(80.0)),
                flex_direction: FlexDirection::Column,
                // Aligns text to the top of the dialogue window.  Yes, it
                // starts from the bottom, so the end is the top!
                justify_content: JustifyContent::FlexEnd,
                // Left-align text.
                align_items: AlignItems::FlexStart,
                ..Default::default()
            },
            // Brown
            material: materials.add(Color::rgba(0.804, 0.522, 0.247, 0.9).into()),
            ..Default::default()
        })
        .with(DialogueWindow {})
        .with_children(|parent| {
            parent.spawn(TextBundle {
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
                    margin: Rect::all(Val::Px(10.0)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with(DialoguePlaceholder {
                handle: level_dialogue,
                ..Default::default()
            })
            .current_entity()
            .map(|entity| transient_state.current_dialogue = Some(entity));
        });
    });

    // Players.
    for i in 0..num_players {
        let texture_handle = to_load.add(asset_server.load(format!("sprites/azuna{}.png", i + 1).as_str()));
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT),
            4,
            8,
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
                    material: transient_state.default_red.clone(),
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
    transient_state: Res<TransientState>,
    mut collider_query: Query<(Entity, &mut Collider, &GlobalTransform, Option<&Handle<Map>>)>,
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

        // check for collisions with objects in current map
        let char_aabb = char_collider.bounding_volume_with_translation(char_global, delta);
        let mut char_collision = Collision::Nil;
        for (collider_entity, collider, collider_global, option_to_map) in collider_query.iter_mut() {
            // TODO: Use the entity instead of the map asset handle in case
            // In theory,  there can be multiple instances of the same map.
            if let Some(owner_map) = option_to_map  {
                if *owner_map != transient_state.current_map {
                    continue;
                }
            }
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
                    match behavior {
                        ColliderBehavior::Obstruct => {}
                        ColliderBehavior::PickUp => {
                            // queue setting collider type to ignore
                            interaction_colliders.insert(collider_entity);
                        }
                        ColliderBehavior::Collect => {}
                        ColliderBehavior::Load { path: _ } => {}
                        ColliderBehavior::Ignore => {}
                    }

                    interaction_event.send(items::Interaction::new(
                        char_entity,
                        collider_entity,
                        behavior.clone(),
                    ));

                    // Upgrade Collision::Nil; don't downgrade Obstruction.
                    match char_collision {
                        Collision::Nil => {
                            char_collision = Collision::Interaction(behavior);
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
        character.collision = char_collision.clone();
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
                        Direction::North => 2,
                        Direction::South => 0,
                        Direction::East => 6,
                        Direction::West => 4,
                    };
                    let num_cells_per_row = 4;

                    match character.state() {
                        CharacterState::Idle    => (1, row * num_cells_per_row + 1),
                        CharacterState::Walking => (4, row * num_cells_per_row),
                        CharacterState::Running => (4, (row + 1) * num_cells_per_row),
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
        // println!("created object {:?}, {:?}", event.map_handle, event.entity);
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
                "biggem" | "gem" => {
                    if !object.visible {
                        ColliderBehavior::Ignore
                    } else {
                        ColliderBehavior::Collect
                    }
                },
                _ => {
                    if object.name.starts_with("load:") {
                        ColliderBehavior::Load { path: object.name[5..].to_string() }
                    } else {
                        if object.is_shape() { // allow hide/show objects without images
                            commands.insert_one(event.entity, Debuggable::default());
                        }
                        ColliderBehavior::Obstruct
                    }
                }
            };

            let collider_component = Collider::new(collider_type, collider_size, Vec2::new(0.0, 0.0));
            commands.insert_one(event.entity, collider_component);
        }
    }
}

fn in_game_start_system(
    commands: &mut Commands,
    mut transient_state: ResMut<TransientState>,
    mut dialogue_events: ResMut<Events<DialogueEvent>>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    query: Query<(Entity, &DialoguePlaceholder), Without<Dialogue>>,
) {
    let should_begin = !transient_state.start_dialogue_shown;
    // Insert a clone of the asset into a new component.
    for (entity, placeholder) in query.iter() {
        let dialogue_asset = dialogue_assets.get(&placeholder.handle).expect("Couldn't find dialogue asset from placeholder handle");
        let mut dialogue = Dialogue::new(placeholder, dialogue_asset.clone());
        if should_begin {
            dialogue.begin("Start", &mut dialogue_events);
            transient_state.start_dialogue_shown = true;
        }
        commands.insert_one(entity, dialogue);
    }
}

fn display_dialogue_system(
    mut event_reader: Local<EventReader<dialogue::DialogueEvent>>,
    dialogue_events: Res<Events<dialogue::DialogueEvent>>,
    mut text_query: Query<&mut Text, With<Dialogue>>,
    mut visible_query: Query<&mut Visible, With<DialogueWindow>>,
) {
    for event in event_reader.iter(&dialogue_events) {
        for mut ui_text in text_query.iter_mut() {
            match event {
                DialogueEvent::End => {
                    ui_text.value = "".to_string();
                    for mut visible in visible_query.iter_mut() {
                        visible.is_visible = false;
                    }
                }
                DialogueEvent::Text(text) => {
                    ui_text.value = text.clone();
                    for mut visible in visible_query.iter_mut() {
                        visible.is_visible = true;
                    }
                }
            }
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
