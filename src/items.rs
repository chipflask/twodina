use std::fs;

use bevy::{asset::FileAssetIo, prelude::*};
use bevy::utils::HashSet;
use bevy_tiled_prototype::Object;

use crate::{
    core::{
        collider::{Collider, ColliderBehavior},
        dialogue::{Dialogue, DialogueEvent},
        game::Game,
        state::{AppState, TransientState},
    },
    loading::LoadProgress,
    scene2d::load_next_map,
};

#[derive(Debug, Default)]
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<ItemInteraction>()
            .add_system_set(SystemSet::on_update(AppState::InGame)
                .with_system(trigger_level_load_system.system()
                    .before("main"))
                .with_system(items_system.system().label("main"))
                .with_system(trigger_dialogue_system.system().label("main"))
                .with_system(inventory_item_reveal_system.system().label("main"))
            );
    }
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub num_gems: u32,
}

// Event to specify that an actor should pick up an item and equip it.
#[derive(Debug)]
pub struct ItemInteraction {
    actor: Entity,
    object: Entity,
    behaviors: HashSet<ColliderBehavior>,
}

impl ItemInteraction {
    pub fn new(actor: Entity, object: Entity, behaviors: HashSet<ColliderBehavior>) -> ItemInteraction {
        // An entity can't pick up itself.
        assert!(actor != object);

        ItemInteraction {
            actor,
            object,
            behaviors,
        }
    }
}

pub fn trigger_level_load_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut interaction_reader: EventReader<ItemInteraction>,
    mut state: ResMut<State<AppState>>,
    mut game_state: ResMut<Game>,
    mut to_load: ResMut<LoadProgress>,
    // mut entity_query: Query<(Entity, &Handle<Map>, &mut Visible, Option<&TileMapChunk>)>,
    // Todo: probably removed when level worflow improved
    transient_state: Res<TransientState>,
) {
    for interaction in interaction_reader.iter() {
        for behavior in interaction.behaviors.iter() {
            match behavior {
                ColliderBehavior::Load { path } => {
                    let level: String = path.to_owned() + &String::from(".tmx");

                    // get asset directory relative to executable - all maps need to be in maps/ for now
                    let mut asset_path = FileAssetIo::get_root_path();
                    asset_path.push("assets");
                    asset_path.push("maps");
                    asset_path.push(level.clone());
                    let level_fs_result = fs::metadata(asset_path.clone());

                    // if this file exists, we're going to want to try loading a state
                    if level_fs_result.is_ok() && state.set(AppState::Loading).is_ok() {
                        debug!("Loading level... {}", level);
                        // eventually do preloading:
                        // game_state.next_map = Some(asset_server.load(level.as_str()));
                        game_state.current_map = to_load.add(asset_server.load(format!("maps/{}", level).as_str()));
                        load_next_map(&mut commands, &mut game_state, &transient_state);
                        to_load.next_state = AppState::InGame;
                        to_load.next_dialogue = Some(path.clone());
                    } else {
                        println!("couldn't load level '{}' as {}", path, asset_path.to_string_lossy());
                    };
                }

                ColliderBehavior::Obstruct |
                ColliderBehavior::Collect |
                ColliderBehavior::Dialogue(_) => {}
            }
        }
    }
}

// handles consume and equip
pub fn items_system(
    mut commands: Commands,
    mut interaction_reader: EventReader<ItemInteraction>,
    mut collider_query: Query<&mut Collider>,
    mut inventory_query: Query<&mut Inventory>,
    object_query: Query<&Object>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    for interaction in interaction_reader.iter() {
        for behavior in interaction.behaviors.iter() {
            match behavior {
                ColliderBehavior::Collect => {
                    commands.entity(interaction.object).despawn_recursive();
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
                    // Prevent getting collected again.
                    if let Ok(mut object_collider) = collider_query.get_mut(interaction.object) {
                        object_collider.remove_behavior(&ColliderBehavior::Collect);
                    }
                }
                ColliderBehavior::Obstruct |
                ColliderBehavior::Load { path: _ } |
                ColliderBehavior::Dialogue(_) => {}
            }
        }
    }
}

pub fn trigger_dialogue_system(
    mut interaction_reader: EventReader<ItemInteraction>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
    mut game: ResMut<Game>,
) {
    for interaction in interaction_reader.iter() {
        for behavior in interaction.behaviors.iter() {
            match behavior {
                ColliderBehavior::Obstruct => {}
                ColliderBehavior::Collect => {}
                ColliderBehavior::Load { path: _ } => {}
                ColliderBehavior::Dialogue(spec) => {
                    if spec.auto_display {
                        for mut dialogue in dialogue_query.iter_mut() {
                            if dialogue.begin_optional(spec.node_name.as_ref(), &mut dialogue_events) {
                                game.dialogue_ui = Some(spec.ui_type);
                            }
                        }
                    }
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
                    collider.insert_behavior(ColliderBehavior::Collect);
                }
            }
        }
    }
}
