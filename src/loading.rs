use bevy::{asset::{Asset, HandleId}, prelude::*, utils::HashSet};
use bevy_tiled_prototype::{Map, MapReadyEvent, Object, ObjectReadyEvent, ObjectShape, PropertyValue};

use crate::{
    core::{
        collider::{Collider, ColliderBehavior},
        dialogue::{Dialogue, DialogueEvent},
        game::{DialogueSpec, DialogueUiType, Game},
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
                state.set(load_progress.next_state).expect("couldn't change state when assets finished loading");
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
        commands.entity(map_entity).remove::<ComplicatedLoad>();
    }
}

pub fn setup_map_objects_system(
    mut commands: Commands,
    mut new_item_query: Query<(&Object, &mut Visible, &Handle<Map>), Without<Collider>>,
    mut game_state: ResMut<Game>,
    mut event_reader: EventReader<ObjectReadyEvent>,
    //mut map_container_query: Query<&mut MapContainer>,
) {
    for event in event_reader.iter() {
        debug!("created object {:?}, {:?}", event.map_handle, event.entity);
        if let Ok((object, mut visible, map_handle)) = new_item_query.get_mut(event.entity) {
            // check if objects already in scene, get default visibility
            let is_visible_option = game_state.entity_visibility.get(&event.entity);
            let mut is_visible = object.visible && !object.is_shape(); // default

            if is_visible_option.is_some() {
                is_visible = is_visible_option.unwrap().clone();
            } else {
                // set default visibility for when map transitions
                game_state
                    .entity_visibility
                    .insert(event.entity.clone(), is_visible);
            }
             // all objects from other maps (or according to last known) should spawn invisible
            if *map_handle != game_state.current_map || !is_visible {
                commands.entity(event.entity).remove::<Draw>();
                visible.is_visible = false;
            }

            let mut behaviors: HashSet<ColliderBehavior> = Default::default();

            let mut has_dialogue = false;
            let mut dialogue_spec = DialogueSpec::default();
            for (k,v) in object.props.iter() {
                if k == "dialogue" {
                    if let PropertyValue::StringValue(s) = v {
                        has_dialogue = true;
                        dialogue_spec.node_name = s.clone();
                        dialogue_spec.ui_type = DialogueUiType::MovementDisabled;
                    }
                } else if k == "notice" {
                    if let PropertyValue::StringValue(s) = v {
                        has_dialogue = true;
                        dialogue_spec.node_name = s.clone();
                        dialogue_spec.ui_type = DialogueUiType::Notice;
                    }
                } else if k == "autodisplay" {
                    if let PropertyValue::BoolValue(b) = v {
                        dialogue_spec.auto_display = *b;
                    }
                }
            }
            if has_dialogue {
                behaviors.insert(ColliderBehavior::Dialogue(dialogue_spec));
            }

            // we should have actual types based on object name
            // and add components based on that
            match object.name.as_ref() {
                "spawn" | "trigger" => {}
                "biggem" | "gem" => {
                    if object.visible {
                        behaviors.insert(ColliderBehavior::Collect);
                    }
                },
                _ => {
                    if object.name.starts_with("load:") {
                        behaviors.insert(ColliderBehavior::Load { path: object.name[5..].to_string() });
                    } else {
                        if object.is_shape() { // allow hide/show objects without images
                            commands.entity(event.entity).insert(Debuggable::default());
                        }
                        behaviors.insert(ColliderBehavior::Obstruct);
                    }
                }
            }

            let collider_size = TILED_MAP_SCALE * match object.shape {
                ObjectShape::Rect { width, height } | ObjectShape::Ellipse { width, height } =>
                    Vec2::new(width, height),
                ObjectShape::Polyline { points: _ } | ObjectShape::Polygon { points: _ } | ObjectShape::Point(_, _) =>
                    Vec2::new(40.0, 40.0),
            };

            let collider_component = Collider::new(behaviors, collider_size, Vec2::new(0.0, 0.0));
            commands.entity(event.entity).insert(collider_component);
        }
    }
}
