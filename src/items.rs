use std::fs;

use bevy::{asset::FileAssetIo, prelude::*};
use bevy::utils::HashSet;
use bevy_tiled_prototype::{Object};

use crate::{
    core::{
        config::Config,
        collider::{Collider, ColliderBehavior},
        dialogue::{Dialogue, DialogueEvent},
        game::{Game, process_script_commands},
        script::ScriptVm,
        state::{AppState, TransientState},
    },
    loading::LoadProgress,
    players::Player,
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
                .with_system(trigger_dialogue_or_script_system.system().label("main"))
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
    config: Res<Config>,
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
                        load_next_map(&mut commands, &mut game_state, &transient_state, &config);
                        to_load.next_state = AppState::InGame;
                        to_load.next_dialogue = Some(path.clone());
                    } else {
                        println!("couldn't load level '{}' as {}", path, asset_path.to_string_lossy());
                    };
                }

                ColliderBehavior::Obstruct |
                ColliderBehavior::Collect |
                ColliderBehavior::Dialogue(_) |
                ColliderBehavior::Ruby(_) => {}
            }
        }
    }
}

// handles consume and equip
pub fn items_system(
    mut commands: Commands,
    mut interaction_reader: EventReader<ItemInteraction>,
    player_query: Query<&Player>,
    // mut inventory_query: Query<&mut Inventory>,
    mut script_vm: NonSendMut<ScriptVm>,
    mut object_query: Query<(&Object, &mut Visible, &mut Collider)>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
) {
    for interaction in interaction_reader.iter() {
        for behavior in interaction.behaviors.iter() {
            match behavior {
                ColliderBehavior::Collect => {
                    commands.entity(interaction.object).despawn_recursive();
                    // if let Ok(mut inventory) = inventory_query.get_mut(interaction.actor) {
                        // inventory.num_gems += 1;

                    // might wish to use type AND name eventually
                    if let Ok((obj, _, _)) = object_query.get_mut(interaction.object) {
                        if let Ok(player) = player_query.get(interaction.actor) {
                            let code = format!("
                                player = game.player_by_id!({})
                                player.trigger(:collect, MapObject.new(name: :{:?}))
                            ", player.id, obj.name);
                            eprintln!("{}", code);
                            script_vm.eval_repl_code_logging_result(code.as_ref());
                        }

                        // Process commands output from the script.
                        process_script_commands(
                            &mut script_vm,
                            &mut object_query,
                            &mut dialogue_query,
                            None,
                            &mut dialogue_events);
                    }
                // }
                    // Prevent getting collected again.
                    if let Ok((_, _, mut object_collider)) = object_query.get_mut(interaction.object) {
                        object_collider.remove_behavior(&ColliderBehavior::Collect);
                    }
                }
                ColliderBehavior::Obstruct |
                ColliderBehavior::Load { path: _ } |
                ColliderBehavior::Dialogue(_) |
                ColliderBehavior::Ruby(_) => {}
            }
        }
    }
}

pub fn trigger_dialogue_or_script_system(
    mut interaction_reader: EventReader<ItemInteraction>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
    mut script_vm: NonSendMut<ScriptVm>,
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
                            if dialogue.begin_optional(spec.node_name.as_ref(), &mut script_vm, &mut dialogue_events) {
                                game.dialogue_ui = Some(spec.ui_type);
                            }
                        }
                    }
                }
                ColliderBehavior::Ruby(code) => {
                    script_vm.eval_repl_code_logging_result(code);
                }
            }
        }
    }
}

pub fn inventory_item_reveal_system() {
    /* ideally this returns early if this level has no items that need monitoring
    mut inventory_query: Query<&mut Inventory>,
    mut object_query: Query<(&Object, &mut Visible, &mut Collider, &Handle<Map>)>,
    mut script_vm: NonSendMut<ScriptVm>,
    game: Res<Game>,
) {
    let mut do_reveal = false;
    let mut total_gems = 0;
    for inventory in inventory_query.iter_mut() {
        total_gems += inventory.num_gems;
    }
    if total_gems >= 4 {
        do_reveal = true;
    }
    if do_reveal {
        for (object, mut visible, mut collider, map_handle) in object_query.iter_mut() {
            // only reveal if it's invisible
            if visible.is_visible {
                continue;
            }
            if total_gems >= 8 && object.name.starts_with("load:") {
                // show hidden portals when you have enough gems
                visible.is_visible = true;
                collider.insert_behavior(ColliderBehavior::Load { path: object.name[5..].to_string() });
                // clear inventory for new map
                for mut items in inventory_query.iter_mut() {
                    items.num_gems = 0;
                    let code = "game.players.each {|pl| pl.num_gems = 0 }";
                    script_vm.eval_repl_code_logging_result(code);
                }
            }

            if object.name == "biggem" && *map_handle == game.current_map {
                visible.is_visible = true;
                collider.behaviors.clear();
                collider.insert_behavior(ColliderBehavior::Collect);
                collider.insert_behavior(ColliderBehavior::Dialogue(
                    DialogueSpec {
                        node_name: "collectedBigGem".to_string(),
                        ui_type: crate::core::game::DialogueUiType::Notice,
                        auto_display: true,
                }));
            }
        }
    }*/
}
