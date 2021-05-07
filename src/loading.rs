
use bevy::{asset::{Asset, HandleId}, prelude::*, utils::HashSet};
use bevy_tiled_prototype::{MapReadyEvent, Object, ObjectReadyEvent, ObjectShape, PropertyValue};

use crate::{
    core::{
        collider::{Collider, ColliderBehavior},
        config::Config,
        dialogue::{Dialogue, DialogueEvent},
        game::{DialogueSpec, DialogueUiType, Game},
        script::ScriptVm,
        state::AppState,
    },
    debug::Debuggable,
    motion::z_from_y,
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
    mut script_vm: NonSendMut<ScriptVm>,
) {
    let handle_ids = load_progress.handles.iter()
        .map(|handle| HandleId::from(handle));
    match asset_server.get_group_load_state(handle_ids) {
        bevy::asset::LoadState::NotLoaded => {}
        bevy::asset::LoadState::Loading => {}
        bevy::asset::LoadState::Loaded => {
            if let Some(node_name) = &load_progress.next_dialogue {
                for mut dialogue in dialogue_query.iter_mut() {
                    dialogue.begin_optional(node_name.as_ref(), &mut script_vm, &mut dialogue_events);
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
        // eventually can be as simple as: commands.insert(map_entity, SpawnedMap);
        // Stop blocking the Loading state transition.
        commands.entity(map_entity).remove::<ComplicatedLoad>();
    }
}

pub fn setup_map_objects_system(
    mut commands: Commands,
    mut new_item_query: Query<(&Object, &mut Visible), Without<Collider>>,
    object_children_query: Query<(&Object, &Children)>,
    mut object_transform_query: Query<(&mut Transform, &Object)>,
    mut game_state: ResMut<Game>,
    mut event_reader: EventReader<ObjectReadyEvent>,
    config: Res<Config>,
    //mut map_container_query: Query<&mut MapContainer>,
) {
    for event in event_reader.iter() {
        let mut objects_to_process: Vec<(Entity, bool)> = vec![(event.entity, false)];
        debug!("created object event received {:?}, {:?}", event.map_handle, event.entity);
        let mut has_kids = false;
        if let Ok((_, children)) = object_children_query.get(event.entity) {
            debug!("found children {:?}", children);
            let mut max_z = f32::MIN;
            for &child in children.iter() {
                if new_item_query.get_mut(child).is_ok() {
                    let parent_transform = object_transform_query.get_mut(event.entity).expect("Need transform for parent").0.clone();
                    let (transform, object) = object_transform_query.get_mut(child).expect("Need transform for child");
                    // println!("{:?}  {:?}", object.size, parent_transform.scale);
                    let maybe_max_z = z_from_y(
                        transform.translation.y + parent_transform.translation.y -
                        object.size.y * parent_transform.scale.y // bottom of the collider
                    );
                    objects_to_process.push((child, true));
                    has_kids = true;
                    if maybe_max_z > max_z {
                        max_z = maybe_max_z;
                    }
                }
            }
            if max_z > f32::MIN {
                let mut parent_transform = object_transform_query.get_mut(event.entity).expect("Need mutable transform for parent").0;
                parent_transform.translation.z = max_z;
            }
        }
        debug!("processing new entities {:?}", objects_to_process);
        for &(entity, is_child) in objects_to_process.iter() {
            if let Ok((object, mut visible)) = new_item_query.get_mut(entity) {
                // check if objects already in scene, get default visibility
                let previous_is_visible = game_state.entity_visibility.get(&entity);

                if !previous_is_visible.is_some() {
                    // set default visibility for when map transitions
                    game_state
                        .entity_visibility
                        .insert(entity.clone(), object.visible && !object.is_shape() // default
                    );
                }
                // all objects from other maps (or according to last known) should spawn invisible
                if event.map_handle != game_state.current_map { // || !is_visible {
                    commands.entity(entity.clone()).remove::<Draw>();
                    visible.is_visible = false;
                }

                let mut behaviors: HashSet<ColliderBehavior> = Default::default();

                let mut has_dialogue = false;
                let mut auto_display_override = None;
                let mut dialogue_spec = DialogueSpec::default();
                for (k,v) in object.props.iter() {
                    if k == "dialogue" {
                        if let PropertyValue::StringValue(s) = v {
                            has_dialogue = true;
                            dialogue_spec.node_name = s.clone();
                            dialogue_spec.ui_type = DialogueUiType::MovementDisabled;
                            dialogue_spec.auto_display = false;
                        }
                    } else if k == "notice" {
                        if let PropertyValue::StringValue(s) = v {
                            has_dialogue = true;
                            dialogue_spec.node_name = s.clone();
                            dialogue_spec.ui_type = DialogueUiType::Notice;
                            dialogue_spec.auto_display = true;
                        }
                    } else if k == "autodisplay" {
                        if let PropertyValue::BoolValue(b) = v {
                            auto_display_override = Some(*b);
                        }
                    }
                }
                if has_dialogue {
                    if let Some(b) = auto_display_override {
                        match dialogue_spec.ui_type {
                            DialogueUiType::MovementDisabled => {
                                eprintln!("Warning: Auto-display of dialogue isn't currently supported")
                            }
                            DialogueUiType::Notice => {
                                dialogue_spec.auto_display = b;
                            }
                        }
                    }
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
                            if object.visible {
                                behaviors.insert(ColliderBehavior::Load { path: object.name[5..].to_string() });
                            }
                        } else {
                            if object.is_shape() { // allow hide/show objects without images
                                commands.entity(entity).insert(Debuggable::default());
                                behaviors.insert(ColliderBehavior::Obstruct);
                            } else if !has_kids {
                                // sprites with no inner objects fully obstruct
                                // we may opt to change this behavior eventually
                                behaviors.insert(ColliderBehavior::Obstruct);
                            }
                        }
                    }
                }

                let mut collider_size = config.map_scale * match object.shape {
                    ObjectShape::Rect { width, height } | ObjectShape::Ellipse { width, height } =>
                        Vec2::new(width, height),
                    ObjectShape::Polyline { points: _ } | ObjectShape::Polygon { points: _ } | ObjectShape::Point(_, _) =>
                        Vec2::new(40.0, 40.0),
                };
                if is_child {
                    let parent = object_transform_query.get_mut(event.entity).expect("Need object for child scale").1;
                    let parent_scale = parent.tile_scale.expect("Need valid scale for child collider");
                    collider_size *= parent_scale;
                }

                let collider_component = Collider::new(behaviors, collider_size, Vec2::new(0.0, 0.0));
                commands.entity(entity).insert(collider_component);
            }
        }
    }
}
