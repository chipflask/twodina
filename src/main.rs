use bevy::{
    prelude::*,
    app::CoreStage::Update,
};
use bevy_tiled_prototype::TiledMapPlugin;

mod actions;
mod camera;
mod core;
mod debug;
mod scene2d;
mod items;
mod loading;
mod motion;
mod players;
mod ui; // in-game ui

use loading::LoadProgress;
use players::Player;

use crate::core::state::{
    AppState,
    EARLY, LATER,
};

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

        .add_plugin(core::menu::MenuPlugin::default())
        .add_plugin(core::dialogue::DialoguePlugin::default())
        .add_plugin(core::input::InputActionPlugin::default())
        .add_plugin(items::ItemsPlugin::default())
        // init
        .add_startup_system(setup_onboot.system()
            .chain(scene2d::initialize_levels_onboot.system())
        )
        // loading
        .on_state_update(LATER, AppState::Loading, loading::wait_for_asset_loading_system.system())
        //
        // menu
        .on_state_update(LATER, AppState::Menu, core::menu::menu_system.system()
            // TODO: run these once using stages
            .chain(players::setup_players_runonce.system())
            .chain(ui::setup_dialogue_window_runonce.system())
        )
        .on_state_update(LATER, AppState::Menu, bevy::input::system::exit_on_esc_system.system())
        .on_state_update(LATER, AppState::Menu, loading::setup_map_objects_system.system())
        .on_state_update(LATER, AppState::Menu, motion::instant_move_player_system.system())

        // in-game:
        .on_state_enter(EARLY, AppState::InGame, scene2d::in_game_start_system.system())
        .on_state_update(EARLY, AppState::InGame, actions::handle_input_system.system())
        .on_state_update(LATER, AppState::InGame, bevy::input::system::exit_on_esc_system.system())
        .on_state_update(LATER, AppState::InGame, camera::update_camera_system.system())
        .on_state_update(LATER, AppState::InGame, debug::position_display_system.system())
        .on_state_update(LATER, AppState::InGame, loading::setup_map_objects_system.system())
        .on_state_update(LATER, AppState::InGame, motion::animate_sprite_system.system())
        .on_state_update(LATER, AppState::InGame, motion::continous_move_character_system.system())
        .on_state_update(LATER, AppState::InGame, motion::instant_move_player_system.system())
        .on_state_update(LATER, AppState::InGame, ui::display_dialogue_system.system())
        .run();
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
