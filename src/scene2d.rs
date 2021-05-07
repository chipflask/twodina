use std::{marker::PhantomData, ops::Mul};

use bevy::{
    asset::FileAssetIo,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_tiled_prototype::{CreatedMapEntities, DebugConfig, LayerData, Map, MapReadyEvent, Object, TileMapChunk, TiledMapBundle, TiledMapCenter};

use crate::{DEBUG_MODE_DEFAULT, core::{
        collider::{Collider, ColliderBehavior},
        config::Config,
        dialogue::{Dialogue, DialogueAsset, DialogueEvent, DialoguePlaceholder},
        game::{DialogueUiType, Game},
        script::ScriptVm,
        state::{AppState, TransientState},
    },
    debug::Debuggable,
    loading::{ComplicatedLoad, LoadProgress}, motion::MoveEntityEvent, players::Player};

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
        dialogue_ui: None,
        current_map: to_load.add(asset_server.load(config.start_map.as_path())),
        current_dialogue: None,
        next_map: None,
        loaded_maps: HashSet::default(),
        entity_visibility: HashMap::default(),
    };

    to_load.next_state = AppState::Menu;
    load_next_map(&mut commands, &mut game_state, &transient_state, &config);

    commands.insert_resource(game_state);
}

pub fn in_game_start_runonce(
    mut commands: Commands,
    mut game_state: ResMut<Game>,
    config: Res<Config>,
    mut script_vm: NonSendMut<ScriptVm>,
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

        let mut script_path = FileAssetIo::get_root_path();
        script_path.push("assets");
        script_path.push(&config.start_script);
        debug!("Loading script: {:?}", script_path.as_os_str());
        script_vm.require_file(&script_path)
            .unwrap_or_else(|e|
                panic!("failed to load startup script: {:?}: {:?}", script_path.as_os_str(), e));

        if should_begin {
            dialogue.begin("Start", &mut script_vm, &mut dialogue_events);
            game_state.start_dialogue_shown = true;
            game_state.dialogue_ui = Some(DialogueUiType::Notice);
        }
        commands.entity(entity).insert(dialogue);
    }
}

pub fn hide_non_map_objects_runonce(
    mut commands: Commands,
    mut game_state: ResMut<Game>,
    mut query: Query<(Entity, &mut Visible, Option<&Handle<Map>>, Option<&TileMapChunk>, Option<&Children>)>,
    // mut state: ResMut<State<AppState>>,
    // mut to_load: ResMut<LoadProgress>,
){
    let mut non_map_entities: Vec<Entity> = Default::default();
    for (entity, _, maybe_map_owner, _, maybe_kids) in query.iter_mut() {
        if let Some(map_owner) = maybe_map_owner {
            if *map_owner != game_state.current_map {
                non_map_entities.push(entity);
                if let Some(children) = maybe_kids {
                    for &child in children.iter() {
                        non_map_entities.push(child);
                    }
                }
            }
        }
    }
    for &entity in non_map_entities.iter(){
        if let Ok((_, mut visible, _, option_chunk, _)) = query.get_mut(entity) {
            // chunks will always be made visible, but objects may have been hidden
            let visible_next_load  = option_chunk.is_some() || visible.is_visible;
            game_state
                .entity_visibility
                .insert(entity.clone(), visible_next_load);
            commands.entity(entity).remove::<Draw>(); // for efficiency (and might help reduce textureId panick)
            visible.is_visible = false;
        }
    }

    if non_map_entities.len() > 0 {
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
    config: &Config,
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
            scale: Vec3::new(config.map_scale, config.map_scale, 1.0),
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


// until bevy_tiled has better support for this, we have to reach in and create objects based on tiles
// TODO: tile objects set up to clear on reload, but MapReadyEvent only runs first load, so they are not re-created
pub fn create_tile_objects_system(
    mut commands: Commands,
    mut map_ready_events: EventReader<MapReadyEvent>,
    query: Query<&MapContainer>,
    maps: Res<Assets<Map>>,
    transient_state: Res<TransientState>,
    mut game_state: ResMut<Game>,
    mut map_query: Query<(&mut CreatedMapEntities, &Handle<Map>)>,
    config: Res<Config>,
) {
    for event in map_ready_events.iter() {
        let map_entity = event.map_entity_option.expect("why didn't you give this map an entity?");
        if let Ok(container) = query.get(map_entity) {
            maps.get(container.asset.clone()).map(|map| {
                let mut templates: HashMap<u32, Object> = Default::default();
                // find all tiles with object layers
                for tileset in map.map.tilesets.iter() {
                    for tile in tileset.tiles.iter() {
                        if let Some(group) = &tile.objectgroup {
                            for obj in &group.objects {
                                templates.insert(tileset.first_gid + tile.id, Object::new(&obj));
                            }
                        }
                    }
                }
                // go through visibile layers for this map and add obstruction objects for tiles
                // NOTE: for now this assumes the entire tile obstructs
                for layer in map.map.layers.iter() {
                    if !layer.visible { continue; }
                    if let LayerData::Finite(tiles) = &layer.tiles {
                        for (tile_y, tilerow) in tiles.iter().enumerate() {
                            for (tile_x, tile) in tilerow.iter().enumerate() {
                                templates.get_mut(&tile.gid).map(|obj| {
                                    obj.position.x = tile_x as f32 * map.tile_size.x;
                                    obj.position.y = tile_y as f32 * map.tile_size.y;
                                    obj.visible = false;
                                    let mut entity_commands = obj.spawn(
                                        &mut commands, None,
                                        &map.map,
                                        container.asset.clone(),
                                        &map.center(
                                        Transform {
                                            translation: Vec3::new(0.0, 0.0, -100.0),
                                            scale: Vec3::new(config.map_scale, config.map_scale, 1.0),
                                            ..Default::default()
                                        }),
                                        &bevy_tiled_prototype::DebugConfig {
                                            enabled: false,
                                            material: Some(transient_state.default_blue.clone()),
                                        }
                                    );
                                    entity_commands
                                        .insert( // for now assume objects in tiles mean entire tile obstructs
                                            Collider::single(
                                                ColliderBehavior::Obstruct,
                                                map.tile_size.clone().mul(config.map_scale),
                                                Vec2::new(0.0, 0.0)
                                            )
                                        )
                                        .insert(Debuggable::default());
                                    // make sure these objects are cleared on auto-reload
                                    for (mut created_map_entities, map_handle) in map_query.iter_mut(){
                                        if container.asset == *map_handle {
                                            for ((_layer_id, _tileset_guid), vec_entities) in created_map_entities.created_layer_entities.iter_mut() {
                                                vec_entities.push(entity_commands.id().clone());
                                                break;  // only need to do this for any one layer in this map (workaround)
                                            }
                                        }
                                    }
                                    // debug objects are invisible by default
                                    game_state.entity_visibility.insert(entity_commands.id(),false);
                                });
                            }
                        }
                    } else {
                        panic!("Infinte maps not supported")
                    }
                }
            });
        }
    }
}
