use std::sync::{Arc, Mutex};

use anyhow;
use artichoke::prelude::*;
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::{BoxedFuture, HashMap},
};

use crate::core::script_core;

#[derive(Default)]
pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<DialogueEvent>()
            .add_asset::<DialogueAsset>()
            .add_asset_loader(DialogueLoader {});
    }
}

// A component that you should spawn with.
#[derive(Debug)]
pub struct DialoguePlaceholder {
    pub handle: Handle<DialogueAsset>,
    pub current_index: usize,
    pub next_index: Option<usize>,
    pub next_node_name: Option<String>,
    pub is_end: bool,
}

// A component that you should insert later once the asset loads.
#[derive(Debug)]
pub struct Dialogue {
    pub handle: Handle<DialogueAsset>,
    pub asset: DialogueAsset,
    pub current_index: usize,
    pub next_index: Option<usize>,
    pub next_node_name: Option<String>,
    pub is_end: bool,
    pub artichoke: Arc<Mutex<Artichoke>>,
}

// This is safe because we wrap the interpreter pointer in Arc<Mutex<T>>.
unsafe impl Send for Dialogue {}
unsafe impl Sync for Dialogue {}

// Event fired by this module so that the app can handle dialogue changes.
#[derive(Debug)]
pub enum DialogueEvent {
    End,
    Text(String),
}

// This is the result of loading the asset file.
#[derive(Clone, Debug, serde::Deserialize, TypeUuid)]
#[uuid = "8571f581-e3b1-4e1c-8d15-6dd81bf8e4e3"]
pub struct DialogueAsset {
    pub name: String,
    pub nodes: Vec<DialogueNode>,
    #[serde(default, skip_serializing)]
    pub nodes_by_name: HashMap<String, usize>,
}

#[derive(Clone, Debug, serde::Deserialize, TypeUuid)]
#[uuid = "df970dd5-6e00-43c3-b85e-f6aa1eab5b26"]
pub struct DialogueNode {
    #[serde(default)]
    pub name: String,
    pub body: NodeBody,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, TypeUuid)]
#[uuid = "fe867e2d-13f8-45f5-9ce7-a078a56b556b"]
pub enum NodeBody {
    Branch(Vec<Choice>),
    // Command(String),
    End,
    GoTo(String),
    Ruby(String),
    Text(String),
}

#[derive(Clone, Debug, serde::Deserialize, TypeUuid)]
#[uuid = "6f55a47b-bf32-4b12-bf41-583785603696"]
pub struct Choice {
    pub text: String,
    pub next: String,
}

#[derive(Default)]
pub struct DialogueLoader;

impl AssetLoader for DialogueLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut asset = ron::de::from_bytes::<DialogueAsset>(bytes)?;
            asset.init();

            load_context.set_default_asset(LoadedAsset::new(asset));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["dialogue"];

        EXTENSIONS
    }
}

impl Default for DialoguePlaceholder {
    fn default() -> Self {
        DialoguePlaceholder {
            handle: Default::default(),
            current_index: 0,
            next_index: None,
            next_node_name: None,
            is_end: true,
        }
    }
}

impl DialogueAsset {
    fn init(&mut self) {
        // When an asset is loaded, build its node to index map.
        let mut map: HashMap<String, usize> = Default::default();
        for (i, node) in self.nodes.iter().enumerate() {
            // If it has no name, don't add it to the map.
            if node.name.is_empty() {
                continue;
            }
            map.insert(node.name.clone(), i);
        }
        self.nodes_by_name = map;
    }
}

impl Dialogue {
    pub fn new(
        placeholder: &DialoguePlaceholder,
        asset: DialogueAsset,
    ) -> Dialogue {
        let mut interpreter = artichoke::interpreter()
            .expect("Couldn't create dialogue interpreter");

        // Add our custom patches.
        script_core::mruby::init(&mut interpreter)
            .expect("failed to initialize ScriptCore");

        Dialogue {
            handle: placeholder.handle.clone(),
            asset,
            current_index: placeholder.current_index,
            next_index: placeholder.next_index,
            next_node_name: placeholder.next_node_name.clone(),
            is_end: placeholder.is_end,
            artichoke: Arc::new(Mutex::new(interpreter)),
        }
    }

    pub fn in_progress(&self) -> bool {
        !self.is_end
    }

    // Start running dialogue from a given node.
    pub fn begin(
        &mut self,
        node_name: &str,
        dialogue_events: &mut EventWriter<DialogueEvent>,
    ) {
        self.next_node_name = Some(node_name.to_string());
        self.is_end = false;
        self.execute(dialogue_events);
    }

    // Start running dialogue from a given node.  If the node doesn't exist, do
    // nothing.
    pub fn begin_optional(
        &mut self,
        node_name: &str,
        dialogue_events: &mut EventWriter<DialogueEvent>,
    ) -> bool {
        if !self.has_node(node_name) {
            return false;
        }
        self.begin(node_name, dialogue_events);

        true
    }

    // Advance the flow of dialogue.  Call this when the player dismisses the
    // current dialogue.
    pub fn advance(
        &mut self,
        dialogue_events: &mut EventWriter<DialogueEvent>,
    ) {
        if self.is_end {
            return;
        }
        // Use next index or increment the current one.
        self.current_index = self
            .next_index
            .unwrap_or_else(|| self.current_index.saturating_add(1));
        self.next_index = None;
        self.execute(dialogue_events);
    }

    pub fn has_node(&self, name: &str) -> bool {
        self.asset.nodes_by_name.contains_key(name)
    }

    // Run and send events so that the app can display text in the UI.
    fn execute(&mut self, dialogue_events: &mut EventWriter<DialogueEvent>) {
        if self.is_end {
            return;
        }

        let dialogue_asset = &self.asset;
        // Override next node with name set in Dialogue::begin().
        if let Some(node_name) = &self.next_node_name {
            match dialogue_asset.nodes_by_name.get(node_name) {
                None => {
                    panic!("Dialogue node with name not found: {}", node_name)
                }
                Some(index) => {
                    self.current_index = *index;
                    self.next_index = None;
                }
            }
        }
        self.next_node_name = None;

        loop {
            match dialogue_asset.nodes.get(self.current_index) {
                None => {
                    // Advanced past the end of all nodes.
                }
                Some(node) => match &node.body {
                    NodeBody::Branch(_) => {
                        panic!("Branches aren't implemented yet");
                    }
                    NodeBody::End => {
                        println!("End");
                        self.is_end = true;
                        self.next_index = None;
                        dialogue_events.send(DialogueEvent::End);
                    }
                    NodeBody::GoTo(name) => {
                        match dialogue_asset.nodes_by_name.get(name) {
                            None => panic!("Dialogue node not found: {}", name),
                            Some(index) => {
                                println!("Going to: {} {}", index, name);
                                self.current_index = *index;
                                continue;
                            }
                        }
                    }
                    NodeBody::Ruby(code) => {
                        let mut interpreter = self.artichoke.lock()
                            .expect("failed to acquire interpreter lock; other thread probably panicked");
                        let result = interpreter.eval(code.as_bytes());
                        println!("Ruby result: {:?}", result);

                        // Continue to next node.
                        self.current_index = self.current_index.saturating_add(1);
                        continue;
                    }
                    NodeBody::Text(text) => {
                        println!("Setting text to: {}", text);
                        dialogue_events.send(DialogueEvent::Text(text.clone()));
                    }
                },
            }
            break;
        }
    }
}
