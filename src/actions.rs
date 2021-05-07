use bevy::prelude::*;

use bevy_tiled_prototype::Map;
use crate::{
    debug::Debuggable,
    core::{
        character::{Character, CharacterState, Direction},
        config::Config,
        dialogue::{Dialogue, DialogueEvent},
        game::{DialogueSpec, Game},
        input::{Action, Flag, InputActionSet},
        script::ScriptVm,
        state::TransientState,
    },
};

use crate::motion::VELOCITY_EPSILON;
use crate::players::Player;

// Something that can trigger dialogue.
#[derive(Debug, Default)]
pub struct DialogueActor {
    // Dialogue that the actor is currently colliding with that could be
    // triggered.
    pub collider_dialogue: Option<DialogueSpec>,
}

pub fn handle_movement_input_system(
    input_actions: Res<InputActionSet>,
    mut transient_state: ResMut<TransientState>,
    game_state: ResMut<Game>,
    mut query: Query<(&mut Character, &Player)>,
    dialogue_query: Query<&Dialogue>,
    mut debuggable: Query<(&mut Visible, Option<&Handle<Map>>), With<Debuggable>>,
    config: Res<Config>,
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

    for dialogue in dialogue_query.iter() {
        if dialogue.in_progress() && game_state.is_in_dialogue() {
            return;
        }
    }

    for (mut character, player) in query.iter_mut() {
        let mut new_direction = None;
        let mut new_velocity = Vec2::ZERO;
        let mut new_state = CharacterState::Idle;
        if input_actions.is_active(Action::Up, player.id) {
            new_direction = Some(Direction::North);
            new_velocity.y = 1.0;
            new_state = CharacterState::Running;
        }
        if input_actions.is_active(Action::Down, player.id) {
            new_direction = Some(Direction::South);
            new_velocity.y = -1.0;
            new_state = CharacterState::Running;
        }

        // Favor facing left or right when two directions are pressed simultaneously
        // by checking left/right after up/down.
        if input_actions.is_active(Action::Left, player.id) {
            new_direction = Some(Direction::West);
            new_velocity.x = -1.0;
            new_state = CharacterState::Running;
        }
        if input_actions.is_active(Action::Right, player.id) {
            new_direction = Some(Direction::East);
            new_velocity.x = 1.0;
            new_state = CharacterState::Running;
        }

        // If the user is pressing two directions at once, go diagonally with
        // unit velocity.
        if !new_velocity.abs_diff_eq(Vec2::ZERO, VELOCITY_EPSILON) {
            new_velocity = new_velocity.normalize();
        }

        if input_actions.is_active(Action::Walk, player.id) {
            character.movement_speed = config.walk_speed;
            new_state = match new_state {
                CharacterState::Running => CharacterState::Walking,
                CharacterState::Idle | CharacterState::Walking => new_state,
            }
        } else {
            character.movement_speed = config.run_speed;
        }

        if let Some(direction) = new_direction {
            character.direction = direction;
        }
        character.velocity.x = new_velocity.x;
        character.velocity.y = new_velocity.y;
        // Don't modify z if the character has a z velocity for some reason.

        character.set_state(new_state);
    }
}

pub fn handle_dialogue_input_system(
    input_actions: Res<InputActionSet>,
    mut game_state: ResMut<Game>,
    mut query: Query<&Player>,
    dialogue_actor_query: Query<&DialogueActor>,
    mut dialogue_query: Query<&mut Dialogue>,
    mut dialogue_events: EventWriter<DialogueEvent>,
    mut script_vm: NonSendMut<ScriptVm>,
) {
    for player in query.iter_mut() {
        if let Some(entity) = game_state.current_dialogue {
            if input_actions.is_active(Action::Accept, player.id) {
                // Advance the current dialogue.
                let mut dialogue = dialogue_query.get_mut(entity).expect("Couldn't find current dialogue entity");
                if dialogue.in_progress() {
                    dialogue.advance(&mut script_vm, &mut dialogue_events);
                    continue;
                }
                // Trigger the dialogue that the player is colliding with.
                for dialogue_actor in dialogue_actor_query.iter() {
                    let mut began = false;
                    if let Some(spec) = &dialogue_actor.collider_dialogue {
                        for mut dialogue in dialogue_query.iter_mut() {
                            if dialogue.begin_optional(spec.node_name.as_ref(), &mut script_vm, &mut dialogue_events) {
                                game_state.dialogue_ui = Some(spec.ui_type);
                                began = true;
                            }
                        }
                    }
                    if began {
                        break;
                    }
                }
            }
        }
    }
}
