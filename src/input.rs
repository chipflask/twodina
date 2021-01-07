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

// Set of gamepads that are currently connected.
#[derive(Default)]
struct GamepadSet {
    gamepads: HashSet<Gamepad>,
    gamepad_event_reader: EventReader<GamepadEvent>,
}

impl Plugin for InputActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_resource(InputActionSet::default())
            .add_resource(GamepadSet::default())
            .add_system_to_stage(stage::EVENT, action_producer_system.system())
            .add_system_to_stage(stage::PRE_UPDATE, gamepad_connection_system.system());
    }
}

fn gamepad_connection_system(
    mut gamepad_set: ResMut<GamepadSet>,
    gamepad_events: Res<Events<GamepadEvent>>,
) {
    for event in gamepad_set.gamepad_event_reader.iter(&gamepad_events) {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                gamepad_set.gamepads.insert(*gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                gamepad_set.gamepads.remove(gamepad);
            }
            GamepadEvent(_, GamepadEventType::AxisChanged(_, _)) => (),
            GamepadEvent(_, GamepadEventType::ButtonChanged(_, _)) => (),
        }
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
    gamepad_set: Res<GamepadSet>,
    axes: Res<Axis<GamepadAxis>>,
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

    for gamepad in gamepad_set.gamepads.iter().cloned() {
        let left_stick_x = axes.get(GamepadAxis(gamepad, GamepadAxisType::LeftStickX)).expect("gamepad axis LeftStickX");
        let left_stick_y = axes.get(GamepadAxis(gamepad, GamepadAxisType::LeftStickY)).expect("gamepad axis LeftStickY");
        if left_stick_x < -0.5 {
            input_action_set.activate(Action::Left);
        }
        if left_stick_x > 0.5 {
            input_action_set.activate(Action::Right);
        }
        if left_stick_y < -0.5 {
            input_action_set.activate(Action::Down);
        }
        if left_stick_y > 0.5 {
            input_action_set.activate(Action::Up);
        }

        let dpad_x = axes.get(GamepadAxis(gamepad, GamepadAxisType::DPadX)).expect("gamepad axis DPadX");
        let dpad_y = axes.get(GamepadAxis(gamepad, GamepadAxisType::DPadY)).expect("gamepad axis DPadY");
        if dpad_x < -0.01 {
            input_action_set.activate(Action::Left);
        }
        if dpad_x > 0.01 {
            input_action_set.activate(Action::Right);
        }
        if dpad_y < -0.01 {
            input_action_set.activate(Action::Down);
        }
        if dpad_y > 0.01 {
            input_action_set.activate(Action::Up);
        }
    }
}
