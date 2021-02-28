use bevy::{prelude::*, utils::{HashMap, HashSet}};
use bevy_tiled_prototype::Map;

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
pub struct Game {
    pub start_dialogue_shown: bool,
    // potentially persisted:
    pub current_dialogue: Option<Entity>, //temporary pub -- move to DialogueState?
    pub current_map: Handle<Map>,
    pub entity_visibility: HashMap<Entity, bool>, // this is a minor memory leak until maps aren't recreated

    pub next_map: Option<Handle<Map>>,
    pub loaded_maps: HashSet<Handle<Map>>,
}
