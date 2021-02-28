use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_tiled_prototype::{DebugConfig, Map, TiledMapBundle, TiledMapCenter};

use crate::{
    core::{
        dialogue::{Dialogue, DialogueAsset, DialogueEvent, DialoguePlaceholder},
        game::Game,
        state::{AppState, TransientState},
    },
    loading::LoadProgress,
    DEBUG_MODE_DEFAULT,
};

// maybe this should go in config.rs or ui.rs?
pub const TILED_MAP_SCALE: f32 = 2.0;

pub fn initialize_levels_onboot(
    In(transient_state): In<TransientState>,
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut to_load: ResMut<LoadProgress>,
    mut query: Query<(Entity, &Handle<Map>, &mut Visible)>,
) {
    let mut game_state = Game {
        start_dialogue_shown: false,
        current_map: to_load.add(asset_server.load("maps/sandyrocks.tmx")),
        current_dialogue: None,
        next_map: None,
        loaded_maps: HashSet::default(),
        entity_visibility: HashMap::default(),
    };

    to_load.next_state = AppState::Menu;
    load_next_map(commands, &mut game_state, &transient_state, &mut query);

    commands.insert_resource(game_state);
    commands.insert_resource(transient_state);
}

pub fn in_game_start_system(
    commands: &mut Commands,
    mut game_state: ResMut<Game>,
    mut dialogue_events: ResMut<Events<DialogueEvent>>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    query: Query<(Entity, &DialoguePlaceholder), Without<Dialogue>>,
) {
    let should_begin = !game_state.start_dialogue_shown;
    // Insert a clone of the asset into a new component.
    for (entity, placeholder) in query.iter() {
        let dialogue_asset = dialogue_assets
            .get(&placeholder.handle)
            .expect("Couldn't find dialogue asset from placeholder handle");
        let mut dialogue = Dialogue::new(placeholder, dialogue_asset.clone());
        if should_begin {
            dialogue.begin("Start", &mut dialogue_events);
            game_state.start_dialogue_shown = true;
        }
        commands.insert_one(entity, dialogue);
    }
}

pub fn load_next_map(
    commands: &mut Commands,
    game_state: &mut Game,
    transient_state: &TransientState,
    query: &mut Query<(Entity, &Handle<Map>, &mut Visible)>,
) {
    for (entity, map_owner, mut visible) in query.iter_mut() {
        if *map_owner != game_state.current_map {
            game_state
                .entity_visibility
                .insert(entity.clone(), visible.is_visible);
            commands.remove_one::<Draw>(entity); // for efficiency (and might help reduce textureId panick)
            visible.is_visible = false;
        } else {
            let is_visible =
                game_state.entity_visibility.get(&entity).unwrap_or(&false);
            // ^ should default object.visible if object
            commands.insert_one(entity, Draw::default());
            visible.is_visible = *is_visible;
        }
    }
    // don't spawn if map already exists
    if game_state.loaded_maps.contains(&game_state.current_map) {
        return;
    }
    commands.spawn(TiledMapBundle {
        map_asset: game_state.current_map.clone(),
        center: TiledMapCenter(true),
        origin: Transform {
            translation: Vec3::new(0.0, 0.0, -100.0),
            scale: Vec3::new(TILED_MAP_SCALE, TILED_MAP_SCALE, 1.0),
            ..Default::default()
        },
        debug_config: DebugConfig {
            enabled: DEBUG_MODE_DEFAULT,
            material: Some(transient_state.default_blue.clone()),
        },
        ..Default::default()
    });
    game_state
        .loaded_maps
        .insert(game_state.current_map.clone());
}
