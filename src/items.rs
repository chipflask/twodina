use std::fs;

use bevy::prelude::*;
use bevy_tiled_prototype::{Map, Object};

use crate::{
    core::{
        collider::{Collider, ColliderBehavior},
        state::{AppState, StageLabels::Early, StageLabels::Later, TransientState},
        game::Game,
    },
    scene2d::load_next_map,
    LoadProgress,
};

#[derive(Debug, Default)]
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<Interaction>()
            .on_state_update(Early, AppState::InGame, trigger_level_load_system.system())
            .on_state_update(Later, AppState::InGame, items_system.system())
            .on_state_update(Later,AppState::InGame, inventory_item_reveal_system.system());
    }
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub num_gems: u32,
}

// Event to specify that an actor should pick up an item and equip it.
#[derive(Debug)]
pub struct Interaction {
    actor: Entity,
    object: Entity,
    behavior: ColliderBehavior,
}

// Transform to apply to an item when it's equipped.
#[derive(Debug)]
pub struct EquippedTransform {
    pub transform: Transform,
}

impl Interaction {
    pub fn new(actor: Entity, object: Entity, behavior: ColliderBehavior) -> Interaction {
        // An entity can't pick up itself.
        assert!(actor != object);

        Interaction {
            actor,
            object,
            behavior,
        }
    }
}

pub fn trigger_level_load_system(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut interaction_reader: EventReader<Interaction>,
    mut state: ResMut<State<AppState>>,
    mut game_state: ResMut<Game>,
    mut to_load: ResMut<LoadProgress>,
    mut entity_query: Query<(Entity, &Handle<Map>, &mut Visible)>,
    // Todo: probably removed when level worflow improved
    transient_state: Res<TransientState>,
) {
    for interaction in interaction_reader.iter() {
        match &interaction.behavior {
            ColliderBehavior::Load { path } => {
                let level: String = path.to_owned() + &String::from(".tmx");
                let level_fs_result = fs::metadata(format!("assets/{}", level));
                // if this file exists, we're going to want to try loading a state
                if level_fs_result.is_ok() && state.set_next(AppState::Loading).is_ok() {
                    println!("Loading level... {}", level);
                    // eventually do preloading:
                    // game_state.next_map = Some(asset_server.load(level.as_str()));
                    game_state.current_map = to_load.add(asset_server.load(level.as_str()));
                    load_next_map(commands, &mut game_state, &transient_state, &mut entity_query);
                    to_load.next_state = AppState::InGame;
                    to_load.next_dialogue = Some(path.clone());
                } else {
                    println!("couldn't load level '{}' at 'assets/{}'", path, level);
                };
            }

            ColliderBehavior::Obstruct | ColliderBehavior::PickUp | ColliderBehavior::Collect | ColliderBehavior::Ignore => {}
        }
    }
}

// handles consume and equip
pub fn items_system(
    commands: &mut Commands,
    mut interaction_reader: EventReader<Interaction>,
    mut query: Query<(&mut Transform, Option<&EquippedTransform>, Option<&mut Collider>)>,
    mut inventory_query: Query<&mut Inventory>,
    object_query: Query<&Object>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    for interaction in interaction_reader.iter() {
        match &interaction.behavior {
            ColliderBehavior::Collect => {
                commands.despawn_recursive(interaction.object);
                if let Ok(mut inventory) = inventory_query.get_mut(interaction.actor) {
                    inventory.num_gems += 1;
                    // might wish to use type AND name eventually
                    if let Ok(obj) = object_query.get(interaction.object) {
                        let sfx_path = match obj.name.as_str() {
                            "biggem" => {
                                inventory.num_gems += 4; // big gems worth 5 - should be param..
                                "sfx/gem_big.ogg"
                            },
                            _ => "sfx/gem_small.ogg"
                        };
                        audio.play(asset_server.load(sfx_path));
                    }
                }
            }
            ColliderBehavior::Obstruct | ColliderBehavior::Ignore | ColliderBehavior::Load { path: _ } => {}
            ColliderBehavior::PickUp => {
                // this is a collectable
                let actor_scale = match query.get_mut(interaction.actor) {
                    Ok((actor_transform, _, _)) => actor_transform.scale.clone(),
                    Err(_) => continue,
                };
                if let Ok((mut object_transform, equipped_transform_option, object_collider_option)) = query.get_mut(interaction.object) {
                    // An object can have a special transform applied when equipped.
                    if let Some(equipped) = equipped_transform_option {
                        object_transform.translation = equipped.transform.translation;
                        object_transform.rotation = equipped.transform.rotation;
                        object_transform.scale = equipped.transform.scale;
                    }
                    object_transform.scale /= actor_scale;
                    // If the object has a Collider component, stop colliding so that it
                    // doesn't get picked up again.
                    if let Some(mut object_collider) = object_collider_option {
                        object_collider.behavior = ColliderBehavior::Ignore;
                    }
                    commands.push_children(interaction.actor, &[interaction.object]);
                }
            }
        }
    }
}

// ideally this returns early if this level has no items that need monitoring
pub fn inventory_item_reveal_system(
    inventory_query: Query<&Inventory>,
    mut object_query: Query<(&Object, &mut Visible, &mut Collider)>,
) {
    for inventory in inventory_query.iter() {
        // if any one character has at least 3 gems, make the big one appear
        if inventory.num_gems >= 3 {
            for (object, mut visible, mut collider) in object_query.iter_mut() {
                if object.name == "biggem" {
                    visible.is_visible = true;
                    collider.behavior = ColliderBehavior::Collect;
                }
            }
        }
    }
}
