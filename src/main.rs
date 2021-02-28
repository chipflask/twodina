use bevy::{
    prelude::*,
    app::CoreStage::Update,
};
use bevy_tiled_prototype::{Map, TiledMapPlugin};

mod camera;
mod character;
mod collider;
mod dialogue;
mod game;
mod input;
mod items;
mod loading;
mod menu; 
mod motion;
mod players;
mod ui; // in-game ui

use character::{Character, CharacterState, Direction};
use dialogue::{Dialogue, DialogueEvent};
use game::GameState;
use loading::LoadProgress;
use input::{Action, Flag, InputActionSet};
use items::Inventory;
use motion::VELOCITY_EPSILON;
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

// run loop stages
pub const EARLY: &str = "EARLY";
pub const LATER: &str = "LATER";

fn main() {
    App::build()
        .insert_resource(State::new(AppState::default()))
        .insert_resource(LoadProgress::default())
        .add_event::<motion::MoveEntityEvent<Player>>()
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
        .on_state_update(LATER, AppState::Loading, loading::wait_for_asset_loading_system.system())
        //
        // menu
        .on_state_update(LATER, AppState::Menu, menu::menu_system.system()
            // TODO: run these once using stages
            .chain(players::setup_players_runonce.system())
            .chain(ui::setup_dialogue_window_runonce.system())
        )
        .on_state_update(LATER, AppState::Menu, bevy::input::system::exit_on_esc_system.system())
        .on_state_update(LATER, AppState::Menu, loading::setup_map_objects_system.system())
        .on_state_update(LATER, AppState::Menu, motion::instant_move_player_system.system())

        // in-game:
        .on_state_enter(EARLY, AppState::InGame, game::in_game_start_system.system())
        .on_state_update(EARLY, AppState::InGame, handle_input_system.system())
        .on_state_update(LATER, AppState::InGame, motion::animate_sprite_system.system())
        .on_state_update(LATER, AppState::InGame, motion::continous_move_character_system.system())
        .on_state_update(LATER, AppState::InGame, camera::update_camera_system.system())
        .on_state_update(LATER, AppState::InGame, position_display_system.system())
        .on_state_update(LATER, AppState::InGame, loading::setup_map_objects_system.system())
        .on_state_update(LATER, AppState::InGame, motion::instant_move_player_system.system())
        .on_state_update(LATER, AppState::InGame, ui::display_dialogue_system.system())
        .on_state_update(LATER, AppState::InGame, bevy::input::system::exit_on_esc_system.system())
        .run();
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
