use bevy::prelude::*;


use bevy_tiled_prototype::Map;
use crate::{
    debug::Debuggable,
    core::{
        character::{RUN_SPEED, WALK_SPEED, Character, CharacterState, Direction},
        dialogue::{Dialogue, DialogueEvent},
        game::Game,
        input::{Action, Flag, InputActionSet},
        state::TransientState,
    },
};

use crate::motion::VELOCITY_EPSILON;
// use crate::items::Inventory;
use crate::players::Player;

// TODO: split between set_velocity_sys and advance_dialogue_sys ?
pub fn handle_input_system(
    input_actions: Res<InputActionSet>,
    mut transient_state: ResMut<TransientState>,
    game_state: ResMut<Game>,
    mut query: Query<(&mut Character, &Player)>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: ResMut<Events<DialogueEvent>>,
    mut debuggable: Query<(&mut Visible, Option<&Handle<Map>>), With<Debuggable>>,
) {
    // check for debug status flag differing from transient_state to determine when to hide/show debug stuff
    if input_actions.has_flag(Flag::Debug) != transient_state.debug_mode {
        transient_state.debug_mode = !transient_state.debug_mode;
        // for now hide, but ideally we spawn debug things here
        for (mut visible, map_option) in debuggable.iter_mut() {
            let mut in_current_map = true;
            map_option.map(|map_handle| {
                in_current_map = *map_handle == game_state.current_map;
            });
            visible.is_visible = in_current_map && transient_state.debug_mode;
        }
    }

    for (mut character, player) in query.iter_mut() {
        let mut new_direction = None;
        let mut new_velocity = Vec2::zero();
        let mut new_state = CharacterState::Idle;
        if input_actions.is_active(Action::Up, player.id) {
            new_direction = Some(Direction::North);
            new_velocity.y = 1.0;
            new_state = CharacterState::Walking;
        }
        if input_actions.is_active(Action::Down, player.id) {
            new_direction = Some(Direction::South);
            new_velocity.y = -1.0;
            new_state = CharacterState::Walking;
        }

        // Favor facing left or right when two directions are pressed simultaneously
        // by checking left/right after up/down.
        if input_actions.is_active(Action::Left, player.id) {
            new_direction = Some(Direction::West);
            new_velocity.x = -1.0;
            new_state = CharacterState::Walking;
        }
        if input_actions.is_active(Action::Right, player.id) {
            new_direction = Some(Direction::East);
            new_velocity.x = 1.0;
            new_state = CharacterState::Walking;
        }

        // If the user is pressing two directions at once, go diagonally with
        // unit velocity.
        if !new_velocity.abs_diff_eq(Vec2::zero(), VELOCITY_EPSILON) {
            new_velocity = new_velocity.normalize();
        }

        if input_actions.is_active(Action::Run, player.id) {
            character.movement_speed = RUN_SPEED;
            new_state = match new_state {
                CharacterState::Walking => CharacterState::Running,
                CharacterState::Idle | CharacterState::Running => new_state,
            }
        } else {
            character.movement_speed = WALK_SPEED;
        }

        if let Some(direction) = new_direction {
            character.direction = direction;
        }
        character.velocity.x = new_velocity.x;
        character.velocity.y = new_velocity.y;
        // Don't modify z if the character has a z velocity for some reason.

        character.set_state(new_state);

        if let Some(entity) = game_state.current_dialogue {
            if input_actions.is_active(Action::Accept, player.id) {
                let mut dialogue = dialogue_query.get_mut(entity).expect("Couldn't find current dialogue entity");
                dialogue.advance(&mut dialogue_events);
            }
        }
    }
}
