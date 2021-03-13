use bevy::{
    prelude::*,
    app::CoreStage::Update,
    app::CoreStage::PreUpdate,
    app::StartupStage::Startup,
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
    StageLabels::Early,
    StageLabels::Later,
    StageLabels::Special,
    TransientState,
};

const DEBUG_MODE_DEFAULT: bool = false;

fn main() {
    App::build()
        .insert_resource(State::new(AppState::default()))
        .insert_resource(LoadProgress::default())
        .add_event::<motion::MoveEntityEvent<Player>>()
        // add stages to run loop
        .add_startup_stage_before(Startup, Early, SystemStage::parallel())
        .add_startup_stage_after(Startup, Later, SystemStage::parallel())
        .add_stage_before(Update, Early, StateStage::<AppState>::default())
        .add_stage_after(Update, Later, StateStage::<AppState>::default())
        .add_stage_after(Later, Special, StateStage::<AppState>::default())
        // add library plugins
        .add_plugins(DefaultPlugins)
        .add_plugin(TiledMapPlugin)
        // add our plugins
        .add_plugin(core::menu::MenuPlugin::default())
        .add_plugin(core::dialogue::DialoguePlugin::default())
        .add_plugin(core::input::InputActionPlugin::default())
        .add_plugin(items::ItemsPlugin::default())
        // initialization
        .add_startup_system_to_stage(Startup, setup_onboot.system())
        .add_startup_system_to_stage(Later, scene2d::initialize_levels_onboot.system())
        // run in all states:
        .add_system_to_stage(Update, bevy::input::system::exit_on_esc_system.system())
        .add_system_to_stage(Update, motion::instant_move_player_system.system())
        .add_system_to_stage(PreUpdate, loading::setup_map_objects_system.system())
        // -- why is "preupdate" required here ^ ? Without it, there's an intermittent bug where colliders aren't added

        // loading
        .on_state_update(Early, AppState::Loading, loading::wait_for_map_ready_system.system())  // this just removes Complicated tag
        .on_state_update(Later, AppState::Loading, loading::wait_for_asset_loading_system.system())
        .on_state_exit(Special, AppState::Loading, scene2d::hide_non_map_objects_runonce.system())

        // menu
        .on_state_update(Later, AppState::Menu, core::menu::menu_system.system()
        // TODO: run these once using stages
            .chain(players::setup_players_runonce.system())
            .chain(ui::setup_dialogue_window_runonce.system())
        )

        // in-game:
        .on_state_enter(Special, AppState::InGame, scene2d::show_map_and_objects_runonce.system())
        .on_state_enter(Special, AppState::InGame, scene2d::in_game_start_runonce.system())
        .on_state_update(Early, AppState::InGame, actions::handle_input_system.system())

        .on_state_update(Later, AppState::InGame, camera::update_camera_system.system())
        .on_state_update(Later, AppState::InGame, debug::position_display_system.system())
        .on_state_update(Later, AppState::InGame, motion::animate_sprite_system.system())
        .on_state_update(Later, AppState::InGame, motion::continous_move_character_system.system())
        .on_state_update(Later, AppState::InGame, ui::display_dialogue_system.system())
        .run();
}

fn setup_onboot(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    camera::initialize_camera(&mut commands);

    // Watch for asset changes.
    asset_server.watch_for_changes().expect("watch for changes");

    let transient_state = TransientState::from_materials(&mut materials, DEBUG_MODE_DEFAULT);
    commands.insert_resource(transient_state);
}
