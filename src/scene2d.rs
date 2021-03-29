use std::marker::PhantomData;

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_tiled_prototype::{DebugConfig, Map, Object, TileMapChunk, TiledMapBundle, TiledMapCenter};

use crate::{DEBUG_MODE_DEFAULT, core::{
        config::Config,
        dialogue::{Dialogue, DialogueAsset, DialogueEvent, DialoguePlaceholder},
        game::Game,
        state::{AppState, TransientState},
    }, loading::{ComplicatedLoad, LoadProgress}, motion::MoveEntityEvent, players::Player};

// maybe this should go in config.rs or ui.rs?
pub const TILED_MAP_SCALE: f32 = 2.0;

#[derive(Default)]
pub struct MapContainer {
    // pub previous_object_visibility: HashMap<Entity, bool>,
    pub asset: Handle<Map>,
}

pub fn initialize_levels_onboot(
    transient_state: Res<TransientState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<Config>,
    mut to_load: ResMut<LoadProgress>,
    // mut query: Query<(Entity, &Handle<Map>, &mut Visible, Option<&TileMapChunk>)>,
) {
    let mut game_state = Game {
        start_dialogue_shown: false,
        current_map: to_load.add(asset_server.load(config.start_map.as_path())),
        current_dialogue: None,
        next_map: None,
        loaded_maps: HashSet::default(),
        entity_visibility: HashMap::default(),
    };

    to_load.next_state = AppState::Menu;
    load_next_map(&mut commands, &mut game_state, &transient_state);

    commands.insert_resource(game_state);
}

pub fn in_game_start_runonce(
    mut commands: Commands,
    mut game_state: ResMut<Game>,
    mut dialogue_events: EventWriter<DialogueEvent>,
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
        commands.entity(entity).insert(dialogue);
    }
}

pub fn hide_non_map_objects_runonce(
    mut commands: Commands,
    mut game_state: ResMut<Game>,
    mut query: Query<(Entity, &Handle<Map>, &mut Visible, Option<&TileMapChunk>)>,
    // mut state: ResMut<State<AppState>>,
    // mut to_load: ResMut<LoadProgress>,
){
    let mut found = false;
    for (entity, map_owner, mut visible, option_chunk) in query.iter_mut() {
        if *map_owner != game_state.current_map {
            // chunks will always be made visible, but objects may have been hidden
            found = true;
            let visible_next_load  = option_chunk.is_some() || visible.is_visible;
            game_state
                .entity_visibility
                .insert(entity.clone(), visible_next_load);
            commands.entity(entity).remove::<Draw>(); // for efficiency (and might help reduce textureId panick)
            visible.is_visible = false;
        }
    }
    if found {
        debug!("Hiding entities not in current map.") // {:?}", game_state.current_map);
    }
}

pub fn show_map_and_objects_runonce(
    mut commands: Commands,
    game_state: Res<Game>,
    mut query: Query<(Entity, &Handle<Map>, &mut Visible, Option<&Object>)>,
    mut move_events: EventWriter<MoveEntityEvent<Player>>,
    // mut state: ResMut<State<AppState>>,
) {
    let mut found = false;
    for (entity, map_owner, mut visible, object_option) in query.iter_mut() {
        if *map_owner == game_state.current_map {
            found = true;
            // objects should have been added to this hashe when processed, default to tile chunks being visible
            // TODO: use flag to ensure object was processed or relocate default/current visibility to marker component
            let is_visible = game_state.entity_visibility.get(&entity).unwrap_or(&true);

            // for now, spawn triggers event whenever we iterate over it in the new map
            if object_option.is_some() && object_option.unwrap().name == "spawn" {
                move_events.send(MoveEntityEvent {
                    object_component: PhantomData,
                    target: entity,
                });
            }
            commands.entity(entity).insert(Draw::default());
            visible.is_visible = *is_visible;
        }
    }
    if found {
        debug!("Show entities for current map."); // {:?}", game_state.current_map);
    }
}

pub fn load_next_map(
    commands: &mut Commands,
    game_state: &mut Game,
    transient_state: &TransientState,
) {
    // don't spawn if map already exists
    if game_state.loaded_maps.contains(&game_state.current_map) {
        return;
    }

    let parent_option = Some(commands.spawn_bundle((
        MapContainer {
            asset: game_state.current_map.clone(),
            ..Default::default()
        },
        // these are necessary for all parent options - TODO: move into bevy_tiled
        Transform::default(),
        GlobalTransform::default(),
        // blocks exit from loading state until spawned
        ComplicatedLoad,
    )).id());

    commands.spawn_bundle(TiledMapBundle {
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
        parent_option,
        ..Default::default()
    });
    game_state
        .loaded_maps
        .insert(game_state.current_map.clone());
}
