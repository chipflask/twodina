use std::marker::PhantomData;

use bevy::{asset::Asset, prelude::*, utils::{HashMap, HashSet}};

use bevy_tiled_prototype::{
    DebugConfig,
    Map,
    Object,
    ObjectReadyEvent,
    ObjectShape,
    TiledMapCenter,
    TiledMapBundle,
};
use crate::{DEBUG_MODE_DEFAULT, Debuggable, collider::{Collider, ColliderBehavior}, dialogue::{Dialogue, DialogueAsset, DialogueEvent, DialoguePlaceholder}, movement::MoveEntityEvent, players::Player};

use crate::{AppState, TransientState};

// maybe this should go in config.rs or ui.rs?
const TILED_MAP_SCALE: f32 = 2.0;


// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
pub struct GameState {
    start_dialogue_shown: bool,
    // potentially persisted:
    pub current_dialogue: Option<Entity>, //temporary pub -- move to DialogueState?
    pub current_map: Handle<Map>,
    entity_visibility: HashMap<Entity, bool>, // this is a minor memory leak until maps aren't recreated

    next_map: Option<Handle<Map>>,
    loaded_maps: HashSet<Handle<Map>>,
}

#[derive(Debug, Default)]
pub struct LoadProgress {
    pub handles: HashSet<HandleUntyped>, // todo, use accessors?
    pub next_state: AppState,
    pub next_dialogue: Option<String>,
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

pub fn initialize_levels_onboot (
        In(transient_state): In<TransientState>,
        commands: &mut Commands,
        asset_server: Res<AssetServer>,
        mut to_load: ResMut<LoadProgress>,
        mut query: Query<(Entity, &Handle<Map>, &mut Visible)>,
) {
    let mut game_state = GameState {
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

pub fn map_item_system(
    commands: &mut Commands,
    new_item_query: Query<&Object>,
    game_state: Res<GameState>,
    mut event_reader: EventReader<ObjectReadyEvent>,
    mut move_events: ResMut<Events<MoveEntityEvent<Player>>>,
    // maps: Res<Assets<Map>>,
) {
    for event in event_reader.iter() {
        if game_state.current_map != event.map_handle {
            continue;
        }
        // println!("created object {:?}, {:?}", event.map_handle, event.entity);
        // let map = maps.get(event.map_handle).expect("Expected to find map from ObjectReadyEvent");
        if let Ok(object) = new_item_query.get(event.entity) {

            // we should have actual types based on object name
            // and add components based on that
            let collider_type = match object.name.as_ref() {
                "spawn" => {
                    move_events.send(MoveEntityEvent {
                        object_component: PhantomData,
                        target: event.entity,
                    });
                    ColliderBehavior::Ignore
                }
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

            let collider_size = TILED_MAP_SCALE * match object.shape {
                ObjectShape::Rect { width, height } | ObjectShape::Ellipse { width, height } =>
                    Vec2::new(width, height),
                ObjectShape::Polyline { points: _ } | ObjectShape::Polygon { points: _ } | ObjectShape::Point(_, _) =>
                    Vec2::new(40.0, 40.0),
            };

            let collider_component = Collider::new(collider_type, collider_size, Vec2::new(0.0, 0.0));
            commands.insert_one(event.entity, collider_component);
        }
    }
}

pub fn in_game_start_system(
    commands: &mut Commands,
    mut game_state: ResMut<GameState>,
    mut dialogue_events: ResMut<Events<DialogueEvent>>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    query: Query<(Entity, &DialoguePlaceholder), Without<Dialogue>>,
) {
    let should_begin = !game_state.start_dialogue_shown;
    // Insert a clone of the asset into a new component.
    for (entity, placeholder) in query.iter() {
        let dialogue_asset = dialogue_assets.get(&placeholder.handle).expect("Couldn't find dialogue asset from placeholder handle");
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
    game_state: &mut GameState,
    transient_state: &TransientState,
    query: &mut Query<(Entity, &Handle<Map>, &mut Visible)>,
) {
    for (entity, map_owner, mut visible) in query.iter_mut() {
        if *map_owner != game_state.current_map {
            game_state.entity_visibility.insert(entity.clone(), visible.is_visible);
            commands.remove_one::<Draw>(entity); // for efficiency (and might help reduce textureId panick)
            visible.is_visible = false;
        } else {
            let is_visible = game_state.entity_visibility.get(&entity).unwrap_or(&false);
            // ^ should default object.visible if object
            commands.insert_one(entity, Draw::default());
            visible.is_visible = *is_visible;
        }
    }
    // don't spawn if map already exists
    if game_state.loaded_maps.contains(&game_state.current_map) {
        return;
    }
    commands
        .spawn(TiledMapBundle {
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
    game_state.loaded_maps.insert(game_state.current_map.clone());
}
