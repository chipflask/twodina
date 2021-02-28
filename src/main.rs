use bevy::{
    asset::HandleId,
    prelude::*,
    utils::HashSet,
    app::CoreStage::Update,
};
use bevy_tiled_prototype::{Map, TiledMapPlugin};

mod character;
mod collider;
mod dialogue;
mod input;
mod camera;
mod game;
mod players;
mod ui; // in-game ui

mod menu; 
mod movement;
mod items;

use character::{Character, CharacterState, Direction, VELOCITY_EPSILON};
use collider::{Collider, ColliderBehavior, Collision};
use dialogue::{Dialogue, DialogueEvent};
use game::{GameState, LoadProgress};
use input::{Action, Flag, InputActionSet};
use items::Inventory;
use players::Player;

const DEBUG_MODE_DEFAULT: bool = false;

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
pub struct TransientState {
    debug_mode: bool,

    default_blue: Handle<ColorMaterial>,
    default_red: Handle<ColorMaterial>,
    button_color: Handle<ColorMaterial>,
    button_hovered_color: Handle<ColorMaterial>,
    button_pressed_color: Handle<ColorMaterial>,
}

// TODO: debug.rs
// Debug entities will be marked with this so that we can despawn them all when
// debug mode is turned off.
#[derive(Debug, Default)]
struct Debuggable;


struct PlayerPositionDisplay {
    player_id: u32,
}

// const MAP_SKEW: f32 = 1.0; // We liked ~1.4, but this should be done with the camera

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

// todo: utils.rs
// Return the Z translation for a given Y translation.  Z determines occlusion.
pub fn z_from_y(y: f32) -> f32 {
    -y / 100.0
}

// run loop stages
pub const EARLY: &str = "EARLY";
pub const LATER: &str = "LATER";

fn main() {
    App::build()
        .insert_resource(State::new(AppState::default()))
        .insert_resource(LoadProgress::default())
        .add_event::<movement::MoveEntityEvent<Player>>()
        // add stages to run loop
        .add_stage_after(Update, EARLY, StateStage::<AppState>::default())
        .add_stage_after(EARLY, LATER, StateStage::<AppState>::default())
        // add plugins
        .add_plugins(DefaultPlugins)
        .add_plugin(TiledMapPlugin)

        .add_plugin(menu::MenuPlugin::default())
        .add_plugin(dialogue::DialoguePlugin::default())
        .add_plugin(input::InputActionPlugin::default())
        .add_plugin(items::ItemsPlugin::default())
        // init
        .add_startup_system(setup_onboot.system()
            .chain(game::initialize_levels_onboot.system())
        )
        // loading
        .on_state_update(LATER, AppState::Loading, wait_for_asset_loading_system.system())
        //
        // menu
        .on_state_update(LATER, AppState::Menu, menu::menu_system.system()
            // TODO: run these once using stages
            .chain(players::setup_players_runonce.system())
            .chain(ui::setup_dialogue_window_runonce.system())
        )
        .on_state_update(LATER, AppState::Menu, bevy::input::system::exit_on_esc_system.system())
        .on_state_update(LATER, AppState::Menu, game::map_item_system.system())
        .on_state_update(LATER, AppState::Menu, movement::move_player_system.system())

        // in-game:
        .on_state_enter(EARLY, AppState::InGame, game::in_game_start_system.system())
        .on_state_update(EARLY, AppState::InGame, handle_input_system.system())
        .on_state_update(LATER, AppState::InGame, movement::animate_sprite_system.system())
        .on_state_update(LATER, AppState::InGame, move_character_system.system())
        .on_state_update(LATER, AppState::InGame, camera::update_camera_system.system())
        .on_state_update(LATER, AppState::InGame, position_display_system.system())
        .on_state_update(LATER, AppState::InGame, game::map_item_system.system())
        .on_state_update(LATER, AppState::InGame, movement::move_player_system.system())
        .on_state_update(LATER, AppState::InGame, ui::display_dialogue_system.system())
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

// split between movement.rs and dialogue / ui.rs ? actions.rs?
fn handle_input_system(
    input_actions: Res<InputActionSet>,
    mut transient_state: ResMut<TransientState>,
    game_state: ResMut<GameState>,
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
                in_current_map = *map_handle == game_state.current_map;
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

        if let Some(entity) = game_state.current_dialogue {
            if input_actions.is_active(Action::Accept, player.id) {
                let mut dialogue = dialogue_query.get_mut(entity).expect("Couldn't find current dialogue entity");
                dialogue.advance(&mut dialogue_events);
            }
        }
    }
}

fn setup_onboot(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) -> TransientState {
    // Default materials
    let default_blue = materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into());
    let default_red = materials.add(Color::rgba(1.0, 0.4, 0.9, 0.8).into());

    camera::initialize_camera_onboot(commands);

    // Watch for asset changes.
    asset_server.watch_for_changes().expect("watch for changes");

    // transient_state: Res<TransientState>,
    let transient_state = TransientState {
        debug_mode: DEBUG_MODE_DEFAULT,

        default_blue: default_blue.clone(),
        default_red: default_red.clone(),
        button_color: materials.add(Color::rgb(0.4, 0.4, 0.9).into()),
        button_hovered_color: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
        button_pressed_color: materials.add(Color::rgb(0.3, 0.3, 0.8).into()),
    };

    transient_state
}

fn move_character_system(
    time: Res<Time>,
    mut interaction_event: ResMut<Events<items::Interaction>>,
    mut char_query: Query<(Entity, &mut Character, &mut Transform, &GlobalTransform)>,
    game_state: Res<GameState>,
    mut collider_query: Query<(Entity, &mut Collider, &GlobalTransform, Option<&Handle<Map>>)>,
) {
    let mut interaction_colliders: HashSet<Entity> = Default::default();
    for (char_entity, mut character, mut transform, char_global) in char_query.iter_mut() {
        let char_collider = collider_query.get_component::<Collider>(char_entity).unwrap().clone();
        if character.velocity.abs_diff_eq(Vec2::zero(), VELOCITY_EPSILON) {
            // Character has zero velocity.  Nothing to do.
            continue;
        }
        let delta: Vec2 = character.velocity * time.delta_seconds() * character.movement_speed;
        // delta.y /= MAP_SKEW;
        // should stay between +- 2000.0

        // check for collisions with objects in current map
        let char_aabb = char_collider.bounding_volume_with_translation(char_global, delta);
        let mut char_collision = Collision::Nil;
        for (collider_entity, collider, collider_global, option_to_map) in collider_query.iter_mut() {
            // TODO: Use the entity instead of the map asset handle in case
            // In theory,  there can be multiple instances of the same map.
            if let Some(owner_map) = option_to_map  {
                if *owner_map != game_state.current_map {
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
                            // queue setting collider type to ignore, stop collisions if we're carrying
                            interaction_colliders.insert(collider_entity);
                        }
                        ColliderBehavior::Collect => {
                            // queue setting collider type to ignore, make sure we don't double-collect
                            interaction_colliders.insert(collider_entity);
                        }
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

fn position_display_system(
    mut character_query: Query<(&Transform, &Player, &Character, &Inventory)>,
    mut text_query: Query<(&mut Text, &PlayerPositionDisplay)>,
) {
    for (char_transform, player, character, inventory) in character_query.iter_mut() {
        for (mut text, ppd) in text_query.iter_mut() {
            if ppd.player_id == player.id {
                text.sections[0].value = format!(
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
