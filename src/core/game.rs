use bevy::{prelude::*, utils::{HashMap, HashSet}};
use bevy_tiled_prototype::{Map, Object};

use super::{collider::{Collider, ColliderBehavior}, dialogue::{Dialogue, DialogueEvent}, script::{SCRIPT_COMMANDS, ScriptVm, ScriptCommand}};

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

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct DialogueSpec {
    pub node_name: String,
    pub ui_type: DialogueUiType,
    pub auto_display: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DialogueUiType {
    MovementDisabled,
    Notice,
}

impl Default for DialogueUiType {
    fn default() -> Self {
        DialogueUiType::MovementDisabled
    }
}

pub fn process_script_commands(
    script_vm: &mut ScriptVm,
    object_query: &mut Query<(&Object, &mut Visible, &mut Collider)>,
    dialogue_query: &mut Query<&mut Dialogue>,
    mut dialogue: Option<&mut Dialogue>,
    dialogue_events: &mut EventWriter<DialogueEvent>,
) {
    let mut commands = SCRIPT_COMMANDS.lock().expect("mutex was poisoned");
    for command in commands.drain(..) {
        match command {
            ScriptCommand::SetVisible(name, new_visible) => {
                for (object, mut visible, _) in object_query.iter_mut() {
                    if object.name == name {
                        visible.is_visible = new_visible;
                    }
                }
            }
            ScriptCommand::SetCollectable(name, _add_or_remove_todo ) => {
                for (object, _, mut collider) in object_query.iter_mut() {
                    if object.name == name {
                        collider.insert_behavior(ColliderBehavior::Collect);
                    }
                }
            }
            ScriptCommand::StartDialogueIfExists(node_name) => {
                for mut dialogue in dialogue_query.iter_mut() {
                    dialogue.begin_optional(node_name.as_ref(), script_vm, dialogue_events);
                }
                if let Some(dialogue) = &mut dialogue {
                    dialogue.begin_optional(node_name.as_ref(), script_vm, dialogue_events);
                }
            }
        }
    }
}
