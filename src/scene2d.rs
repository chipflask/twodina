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
        dialogue::{Dialogue, DialogueEvent},
        game::{DialogueUiType, Game},
        script::{SCRIPT_COMMANDS, ScriptCommand, ScriptCommandEvent, ScriptVm},
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
    mut script_vm: NonSendMut<ScriptVm>,
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

    // Load startup script.
    let mut script_path = FileAssetIo::get_root_path();
    script_path.push("assets");
    script_path.push(&config.start_script);
    debug!("Loading script: {:?}", script_path.as_os_str());
    script_vm.require_file(&script_path)
        .unwrap_or_else(|e|
            panic!("failed to load startup script: {:?}: {:?}", script_path.as_os_str(), e));

    // Instantiate the game.
    let code = format!("
        unless defined?(game)
          game = Game.new
        end
    ");
    script_vm.eval_repl_code_logging_result(code.as_ref());

    to_load.next_state = AppState::Menu;
    load_next_map(&mut commands, &mut game_state, &transient_state, &config);

    commands.insert_resource(game_state);
}

pub fn in_game_start_runonce(
    mut game_state: ResMut<Game>,
    player_query: Query<&Player>,
    mut script_vm: NonSendMut<ScriptVm>,
    mut script_command_events: EventWriter<ScriptCommandEvent>,
) {
    let should_begin = !game_state.start_dialogue_shown;
    if should_begin {
        let player_str_ids = player_query.iter()
            .map(|player| player.id.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        let code = format!("
            game.trigger_new_game([{}])
        ", player_str_ids);
        script_vm.eval_repl_code_logging_result(code.as_ref());

        game_state.start_dialogue_shown = true;
        game_state.dialogue_ui = Some(DialogueUiType::Notice);
    }

    // Make events for script commands.
    let mut commands = SCRIPT_COMMANDS.lock().expect("mutex poisoned");
    for command in commands.drain(..) {
        script_command_events.send(ScriptCommandEvent::new(command));
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

pub fn trigger_map_enter_script_event_system(
    mut map_ready_events: EventReader<MapReadyEvent>,
    query: Query<&MapContainer>,
    game_state: Res<Game>,
    mut script_vm: NonSendMut<ScriptVm>,
    asset_server: Res<AssetServer>,
) {
    for event in map_ready_events.iter() {
        let map_entity = event.map_entity_option.expect("expected map ready event to have a map entity");
        if let Ok(container) = query.get(map_entity) {
            // Notify the script that the map has loaded and set the current
            // map.
            //
            // TODO: This always returns None, and I don't know why.
            let map_path = asset_server.get_handle_path(container.asset.clone());
            let code = format!("
                    map = game.find_or_create_map(id: {}, filename: {})
                    map.trigger(:load)
                    game.trigger_enter_map(map) if {}
                ",
                map_entity.to_bits(),
                match map_path {
                    None => "nil".to_string(),
                    Some(asset_path) => {
                        format!("{:?}", asset_path.path().to_string_lossy())
                    }
                },
                game_state.current_map == container.asset,
            );
            eprintln!("running:\n{}", code);
            script_vm.eval_repl_code_logging_result(code.as_ref());
        }
    }
}

pub fn process_script_commands_system(
    mut script_vm: NonSendMut<ScriptVm>,
    mut object_query: Query<(&Object, &mut Visible, &mut Collider)>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
    mut script_command_events: EventReader<ScriptCommandEvent>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    for event in script_command_events.iter() {
        match &event.command {
            ScriptCommand::SetVisible(name, new_visible) => {
                for (object, mut visible, _) in object_query.iter_mut() {
                    if object.name == *name {
                        visible.is_visible = *new_visible;
                    }
                }
            }
            ScriptCommand::SetCollectable(name, _add_or_remove_todo ) => {
                for (object, _, mut collider) in object_query.iter_mut() {
                    if object.name == *name {
                        collider.insert_behavior(ColliderBehavior::Collect);
                    }
                }
            }
            ScriptCommand::StartDialogueIfExists(node_name) => {
                for mut dialogue in dialogue_query.iter_mut() {
                    dialogue.begin_optional(node_name.as_ref(),
                                            &mut script_vm,
                                            &mut dialogue_events);
                }
            }
            ScriptCommand::PlaySound(sfx_path) => {
                audio.play(asset_server.load(sfx_path.as_str()));
            }
        }
    }
}
