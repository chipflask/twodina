use bevy::{asset::{Asset, HandleId}, prelude::*, utils::HashSet};
use bevy_tiled_prototype::{MapReadyEvent, Object, ObjectReadyEvent, ObjectShape};

use crate::{
    core::{
        collider::{Collider, ColliderBehavior},
        dialogue::{Dialogue, DialogueEvent},
        game::Game,
        state::AppState,
    },
    debug::Debuggable,
    scene2d::TILED_MAP_SCALE,
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

// prevents existing Loading state just from asset load (e.g. map needs spawn)
pub struct ComplicatedLoad;

pub fn wait_for_asset_loading_system(
    mut state: ResMut<State<AppState>>,
    mut load_progress: ResMut<LoadProgress>,
    asset_server: Res<AssetServer>,
    loading_map_query: Query<Entity, With<ComplicatedLoad>>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
) {
    let handle_ids = load_progress.handles.iter()
        .map(|handle| HandleId::from(handle));
    match asset_server.get_group_load_state(handle_ids) {
        bevy::asset::LoadState::NotLoaded => {}
        bevy::asset::LoadState::Loading => {}
        bevy::asset::LoadState::Loaded => {
            if let Some(node_name) = &load_progress.next_dialogue {
                for mut dialogue in dialogue_query.iter_mut() {
                    dialogue.begin_optional(node_name.as_ref(), &mut dialogue_events);
                }
            }
            // block transition if there are any complicated load objects still in the queue
            if loading_map_query.iter().next().is_none() {
                state.set_next(load_progress.next_state).expect("couldn't change state when assets finished loading");
                load_progress.reset();
            }
        }
        // TODO: Handle failed loading of assets, get rid of fs check in items.rs
        bevy::asset::LoadState::Failed => {}
    }
}


pub fn wait_for_map_ready_system(
    mut commands: Commands,
    mut map_ready_events: EventReader<MapReadyEvent>,
) {
    for event in map_ready_events.iter() {
        let map_entity = event.map_entity_option.expect("why didn't you give this map an entity?");
        // commands.insert(map_entity, SpawnedMap);
        // Stop blocking the Loading state transition.
        commands.remove::<ComplicatedLoad>(map_entity);
    }
}

pub fn setup_map_objects_system(
    mut commands: Commands,
    mut new_item_query: Query<(&Object, &mut Visible), Without<Collider>>,
    mut game_state: ResMut<Game>,
    mut event_reader: EventReader<ObjectReadyEvent>,
    //mut map_container_query: Query<&mut MapContainer>,
) {
    for event in event_reader.iter() {
        debug!("created object {:?}, {:?}", event.map_handle, event.entity);
        if let Ok((object, mut visible)) = new_item_query.get_mut(event.entity) {
            // set default visibility for when map transitions
            game_state
                .entity_visibility
                .insert(event.entity.clone(), object.visible && !object.is_shape());
            // all objects SHOULD start invisible by default
            commands.remove::<Draw>(event.entity);
            visible.is_visible = false;

            // we should have actual types based on object name
            // and add components based on that
            let collider_type = match object.name.as_ref() {
                "spawn" => {
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
