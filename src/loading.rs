use std::marker::PhantomData;

use bevy::{
    prelude::*,
    utils::HashSet,
    asset::{Asset, HandleId}, 
};
use bevy_tiled_prototype::{Object, ObjectReadyEvent, ObjectShape};

use crate::{
    core::{
        collider::{Collider, ColliderBehavior},
        dialogue::{Dialogue, DialogueEvent},
        game::Game,
        state::AppState,
    },
    debug::Debuggable,
    scene2d::TILED_MAP_SCALE,
    motion::MoveEntityEvent,
    players::Player,
};

#[derive(Debug, Default)]
pub struct LoadProgress {
    handles: HashSet<HandleUntyped>,
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

pub fn wait_for_asset_loading_system(
    mut state: ResMut<State<AppState>>,
    mut load_progress: ResMut<LoadProgress>,
    asset_server: Res<AssetServer>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
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

pub fn setup_map_objects_system(
    mut commands: Commands,
    new_item_query: Query<&Object>,
    game_state: Res<Game>,
    mut event_reader: EventReader<ObjectReadyEvent>,
    mut move_events: EventWriter<MoveEntityEvent<Player>>,
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
                            commands.insert(event.entity, Debuggable::default());
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
            commands.insert(event.entity, collider_component);
        }
    }
}
