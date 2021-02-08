use bevy::{prelude::*, reflect::TypeUuid, utils::HashMap};

use bevy_asset_ron::RonAssetPlugin;

use crate::{AppState, LATER};

#[derive(Default)]
pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<DialogueChangeEvent>()
            .add_plugin(RonAssetPlugin::<DialogueAsset>::new(&["dialogue"]))
            .on_state_update(LATER, AppState::InGame, asset_load_system.system())
            .on_state_update(LATER, AppState::InGame, handle_change_system.system())
            .on_state_update(LATER, AppState::InGame, display_dialogue_system.system());
    }
}

// A component that you should spawn with.
#[derive(Debug, Default)]
pub struct Dialogue {
    pub handle: Handle<DialogueAsset>,
    pub current_index: usize,
    pub next_index: Option<usize>,
}

// Event fired when the current index changes for a given Dialogue.
#[derive(Debug)]
pub struct DialogueChangeEvent {
    // Entity with the Dialogue component.
    pub entity: Entity,
}

// This is the result of loading the asset file.
#[derive(Debug, serde::Deserialize, TypeUuid)]
#[uuid = "8571f581-e3b1-4e1c-8d15-6dd81bf8e4e3"]
pub struct DialogueAsset {
    pub name: String,
    pub nodes: Vec<DialogueNode>,
    #[serde(default, skip_serializing)]
    pub nodes_by_name: HashMap<String, usize>,
}

#[derive(Debug, serde::Deserialize, TypeUuid)]
#[uuid = "df970dd5-6e00-43c3-b85e-f6aa1eab5b26"]
pub struct DialogueNode {
    #[serde(default)]
    pub name: String,
    pub body: NodeBody,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, serde::Deserialize, TypeUuid)]
#[uuid = "fe867e2d-13f8-45f5-9ce7-a078a56b556b"]
pub enum NodeBody {
    Branch(Vec<Choice>),
    // Command(String),
    GoTo(String),
    Text(String),
}

#[derive(Debug, serde::Deserialize, TypeUuid)]
#[uuid = "6f55a47b-bf32-4b12-bf41-583785603696"]
pub struct Choice {
    pub text: String,
    pub next: String,
}

impl Dialogue {
    pub fn advance(&mut self) {
        match self.next_index {
            None => {
                let index = self.current_index.saturating_add(1);
                self.current_index = index;
            }
            Some(index) => {
                self.current_index = index;
            }
        }
        self.next_index = None;
    }
}

fn asset_load_system(
    mut event_reader: Local<EventReader<AssetEvent<DialogueAsset>>>,
    dialogue_events: Res<Events<AssetEvent<DialogueAsset>>>,
    mut dialogue_assets: ResMut<Assets<DialogueAsset>>,
    mut dialogue_change_events: ResMut<Events<DialogueChangeEvent>>,
    query: Query<Entity, With<Dialogue>>,
) {
    for event in event_reader.iter(&dialogue_events) {
        match event {
            AssetEvent::Created { handle } => {
                // When an asset is loaded, build its node to index map.
                let mut dialogue_asset = dialogue_assets.get_mut(handle).expect("Couldn't find dialogue asset from event handle");
                let mut map: HashMap<String, usize> = Default::default();
                for (i, node) in dialogue_asset.nodes.iter().enumerate() {
                    // If it has no name, don't add it to the map.
                    if node.name.is_empty() {
                        continue;
                    }
                    map.insert(node.name.clone(), i);
                }
                dialogue_asset.nodes_by_name = map;
                println!("{:#?}", dialogue_asset);
                // Find all the entities referring to it, and send a change
                // event for the initial node.
                for entity in query.iter() {
                    println!("Sending event {:?}", entity);
                    dialogue_change_events.send(DialogueChangeEvent {
                        entity,
                    })
                }
            }
            AssetEvent::Modified { handle: _ } => (),
            AssetEvent::Removed { handle: _ } => (),
        }
    }
}

// When the dialogue is mutated, send an event.
fn handle_change_system(
    query: Query<Entity, Mutated<Dialogue>>,
    mut dialogue_change_events: ResMut<Events<DialogueChangeEvent>>,
) {
    for entity in query.iter() {
        println!("Sending event due to change {:?}", entity);
        dialogue_change_events.send(DialogueChangeEvent {
            entity,
        })
    }
}

// When a dialogue component changes its current node, update the text display.
fn display_dialogue_system(
    mut event_reader: Local<EventReader<DialogueChangeEvent>>,
    dialogue_change_events: Res<Events<DialogueChangeEvent>>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut query: Query<(&mut Text, &mut Dialogue)>,
) {
    for _event in event_reader.iter(&dialogue_change_events) {
        println!("Got change event");
        for (mut ui_text, mut dialogue) in query.iter_mut() {
            println!("Found dialogue entity with text UI");
            let dialogue_asset = dialogue_assets.get(dialogue.handle.clone()).expect("Couldn't find dialogue asset from component handle");
            loop {
                match &dialogue_asset.nodes.get(dialogue.current_index) {
                    None => {
                        // Advanced past the end of all nodes.
                    }
                    Some(node) => {
                        match &node.body {
                            NodeBody::Branch(_) => {
                                panic!("Branches aren't implemented yet");
                            }
                            NodeBody::GoTo(name) => {
                                match dialogue_asset.nodes_by_name.get(name) {
                                    None => panic!("Dialogue node not found: {}", name),
                                    Some(index) => {
                                        println!("Going to: {} {}", index, name);
                                        dialogue.current_index = *index;
                                        continue;
                                    }
                                }
                            }
                            NodeBody::Text(text) => {
                                println!("Setting text to: {}", text);
                                ui_text.value = text.clone();
                            }
                        }
                    }
                }
                break;
            }
        }
    }
}
