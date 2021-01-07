// use std::collections::HashSet;
use bevy::prelude::*;
use bevy::utils::{AHashExt, HashSet};

// Add this plugin to your app.
#[derive(Debug, Default)]
pub struct InputActionPlugin {}

// When handling actions, your system will use this as a resource to query for
// actions.
#[derive(Debug)]
pub struct InputActionSet {
    actions: HashSet<Action>,
}

// The application actions.  Raw input like keyboard key presses are mapped to
// these.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
}

impl Plugin for InputActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_resource(InputActionSet::default())
            .add_system_to_stage(stage::EVENT, action_producer_system.system());
    }
}

impl Default for InputActionSet {
    fn default() -> Self {
        InputActionSet {
            actions: HashSet::with_capacity(8)
        }
    }
}

impl InputActionSet {
    pub fn is_active(&self, action: Action) -> bool {
        self.actions.contains(&action)
    }

    fn activate(&mut self, action: Action) {
        self.actions.insert(action);
    }

    fn clear(&mut self) {
        self.actions.clear();
    }
}

fn action_producer_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut input_action_set: ResMut<InputActionSet>,
) {
    input_action_set.clear();
    if keyboard_input.pressed(KeyCode::W) {
        input_action_set.activate(Action::Up);
    }
    if keyboard_input.pressed(KeyCode::A) {
        input_action_set.activate(Action::Left);
    }
    if keyboard_input.pressed(KeyCode::S) {
        input_action_set.activate(Action::Down);
    }
    if keyboard_input.pressed(KeyCode::D) {
        input_action_set.activate(Action::Right);
    }
}
