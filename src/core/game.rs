use bevy::{prelude::*, utils::{HashMap, HashSet}};
use bevy_tiled_prototype::Map;

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
pub struct Game {
    pub start_dialogue_shown: bool,
    // TODO: Nothing currently sets this back to None.  Make sure to check if
    // the dialogue is in progress also.
    pub dialogue_ui: Option<DialogueUiType>,

    // potentially persisted:
    pub current_dialogue: Option<Entity>, //temporary pub -- move to DialogueState?
    pub current_map: Handle<Map>,
    pub entity_visibility: HashMap<Entity, bool>, // this is a minor memory leak until maps aren't recreated

    pub next_map: Option<Handle<Map>>,
    pub loaded_maps: HashSet<Handle<Map>>,
}

impl Game {
    pub fn is_in_dialogue(&self) -> bool {
        match self.dialogue_ui {
            Some(DialogueUiType::MovementDisabled) => true,
            Some(DialogueUiType::Notice) => false,
            None => false,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DialogueSpec {
    pub node_name: String,
    pub ui_type: DialogueUiType,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DialogueUiType {
    MovementDisabled,
    Notice,
}
