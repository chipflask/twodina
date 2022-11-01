use bevy::prelude::*;

use crate::core::character::{Character, CharacterState, Direction};


pub(crate) fn keyboard_input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Character>,
) {
    if keyboard_input.just_pressed(KeyCode::W) {
        for mut character in query.iter_mut() {
            character.direction = Direction::North;
            character.state = CharacterState::Walking;
            character.velocity.x = 0.0;
            character.velocity.y = 1.0;
        }
    }
    if keyboard_input.just_pressed(KeyCode::A) {
        for mut character in query.iter_mut() {
            character.direction = Direction::West;
            character.state = CharacterState::Walking;
            character.velocity.x = -1.0;
            character.velocity.y = 0.0;
        }
    }
    if keyboard_input.just_pressed(KeyCode::S) {
        for mut character in query.iter_mut() {
            character.direction = Direction::South;
            character.state = CharacterState::Walking;
            character.velocity.x = 0.0;
            character.velocity.y = -1.0;
        }
    }
    if keyboard_input.just_pressed(KeyCode::D) {
        for mut character in query.iter_mut() {
            character.direction = Direction::East;
            character.state = CharacterState::Walking;
            character.velocity.x = 1.0;
            character.velocity.y = 0.0;
        }
    }
    if keyboard_input.just_released(KeyCode::W)
        || keyboard_input.just_released(KeyCode::A)
        || keyboard_input.just_released(KeyCode::S)
        || keyboard_input.just_released(KeyCode::D)
    {
        for mut character in query.iter_mut() {
            if !keyboard_input.pressed(KeyCode::W)
                && !keyboard_input.pressed(KeyCode::S)
            {
                character.velocity.y = 0.0;
            }
            if !keyboard_input.pressed(KeyCode::A)
                && !keyboard_input.pressed(KeyCode::D)
            {
                character.velocity.x = 0.0;
            }
            // disable animation if no longer moving
            if character.velocity.distance(Vec3::ZERO) < 0.01 {
                character.make_idle();
            }
        }
    }
}