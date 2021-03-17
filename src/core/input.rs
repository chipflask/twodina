use std::convert::TryFrom;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy::app::CoreStage::{First, PreUpdate};

// Add this plugin to your app.
#[derive(Debug, Default)]
pub struct InputActionPlugin {}

// When handling actions, your system will use this as a resource to query for
// actions.
#[derive(Debug)]
pub struct InputActionSet {
    actions: HashSet<(Action, u32)>,
    flags: HashSet<Flag>,
}

// The application actions.  Raw input like keyboard key presses are mapped to
// these.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,

    Run,

    Accept,
}

// inputs that toggle values on key/button press map to these
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Flag {
    Debug,
    // Sneak
}

// Set of gamepads that are currently connected.
#[derive(Default)]
struct GamepadSet {
    gamepads: HashSet<Gamepad>,
}

impl Plugin for InputActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .insert_resource(InputActionSet::default())
            .insert_resource(GamepadSet::default())
            .add_system_to_stage(First, action_producer_system.system())
            .add_system_to_stage(PreUpdate, gamepad_connection_system.system());
    }
}

fn gamepad_connection_system(
    mut gamepad_set: ResMut<GamepadSet>,
    mut gamepad_events: EventReader<GamepadEvent>,
) {
    for event in gamepad_events.iter() {
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
            actions: HashSet::default(),
            flags: HashSet::default(),
        }
    }
}

impl InputActionSet {
    pub fn is_active(&self, action: Action, player: u32) -> bool {
        self.actions.contains(&(action, player))
    }
    pub fn has_flag(&self, flag: Flag) -> bool {
        self.flags.contains(&flag)
    }

    fn activate(&mut self, action: Action, player: u32) {
        self.actions.insert((action, player));
    }

    fn clear(&mut self) {
        self.actions.clear();
    }

    fn toggle(&mut self, flag: Flag) {
        if self.flags.contains(&flag) {
            self.flags.remove(&flag);
        } else {
            self.flags.insert(flag);
        }
    }
}

fn action_producer_system(
    keyboard_input: Res<Input<KeyCode>>,
    gamepad_set: Res<GamepadSet>,
    button_inputs: Res<Input<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
    mut input_action_set: ResMut<InputActionSet>,
) {
    input_action_set.clear();

    if keyboard_input.just_pressed(KeyCode::Space) {
        input_action_set.activate(Action::Accept, 0);
    }

    if keyboard_input.just_released(KeyCode::F3) {
        input_action_set.toggle(Flag::Debug);
    }

    if keyboard_input.pressed(KeyCode::W) {
        input_action_set.activate(Action::Up, 0);
    }
    if keyboard_input.pressed(KeyCode::A) {
        input_action_set.activate(Action::Left, 0);
    }
    if keyboard_input.pressed(KeyCode::S) {
        input_action_set.activate(Action::Down, 0);
    }
    if keyboard_input.pressed(KeyCode::D) {
        input_action_set.activate(Action::Right, 0);
    }
    if keyboard_input.pressed(KeyCode::LShift) {
        input_action_set.activate(Action::Run, 0);
    }

    if keyboard_input.pressed(KeyCode::Up) {
        input_action_set.activate(Action::Up, 1);
    }
    if keyboard_input.pressed(KeyCode::Left) {
        input_action_set.activate(Action::Left, 1);
    }
    if keyboard_input.pressed(KeyCode::Down) {
        input_action_set.activate(Action::Down, 1);
    }
    if keyboard_input.pressed(KeyCode::Right) {
        input_action_set.activate(Action::Right, 1);
    }
    if keyboard_input.pressed(KeyCode::RShift) {
        input_action_set.activate(Action::Run, 1);
    }

    for (i, gamepad) in gamepad_set.gamepads.iter().cloned().enumerate() {
        let left_stick_x = axes
            .get(GamepadAxis(gamepad, GamepadAxisType::LeftStickX))
            .expect("gamepad axis LeftStickX");
        let left_stick_y = axes
            .get(GamepadAxis(gamepad, GamepadAxisType::LeftStickY))
            .expect("gamepad axis LeftStickY");

        let player_num = u32::try_from(i).expect("brah how many controllers u got?");

        if left_stick_x < -0.5 {
            input_action_set.activate(Action::Left, player_num);
        }
        if left_stick_x > 0.5 {
            input_action_set.activate(Action::Right, player_num);
        }
        if left_stick_y < -0.5 {
            input_action_set.activate(Action::Down, player_num);
        }
        if left_stick_y > 0.5 {
            input_action_set.activate(Action::Up, player_num);
        }

        let dpad_x = axes
            .get(GamepadAxis(gamepad, GamepadAxisType::DPadX))
            .expect("gamepad axis DPadX");
        let dpad_y = axes
            .get(GamepadAxis(gamepad, GamepadAxisType::DPadY))
            .expect("gamepad axis DPadY");
        if dpad_x < -0.01 {
            input_action_set.activate(Action::Left, player_num);
        }
        if dpad_x > 0.01 {
            input_action_set.activate(Action::Right, player_num);
        }
        if dpad_y < -0.01 {
            input_action_set.activate(Action::Down, player_num);
        }
        if dpad_y > 0.01 {
            input_action_set.activate(Action::Up, player_num);
        }

        if button_inputs.pressed(GamepadButton(gamepad, GamepadButtonType::West)) {
            input_action_set.activate(Action::Run, player_num);
        }
    }
}
