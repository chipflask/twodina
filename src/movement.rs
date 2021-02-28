use std::marker::PhantomData;
use std::convert::TryFrom;

use crate::{
    character::{AnimatedSprite, Character, CharacterState, Direction},
    players::{Player, PLAYER_WIDTH}
};
use bevy::prelude::*;

pub struct MoveEntityEvent<T: Component> {
    pub object_component: PhantomData<T>,
    pub target: Entity,
}

pub fn move_player_system(
    events: EventReader<MoveEntityEvent<Player>>,
    query: Query<(&mut Transform, Option<&Player>)>,
) {
    move_entity(events, query, Vec3::new(2.2 * PLAYER_WIDTH, 0.0, 0.0));
}

fn move_entity<T: Component>(
    mut events: EventReader<MoveEntityEvent<T>>,
    mut query: Query<(&mut Transform, Option<&T>)>,
    offset: Vec3, // additive
) {
    for event in events.iter() {
        let target = match query.get_mut(event.target) {
            Ok((transform, _)) => transform.translation,
            Err(_) => continue,
        };
        let mut total_offset = Vec3::zero();
        for (mut transform, has_component) in query.iter_mut() {
            if has_component.is_none() {
                continue;
            }
            transform.translation = target + total_offset;
            total_offset += offset;
        }
    }
}

pub fn animate_sprite_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(&mut TextureAtlasSprite, &Handle<TextureAtlas>, &mut AnimatedSprite, Option<&Character>)>
) {
    for (mut sprite, texture_atlas_handle, mut animated_sprite, character_option) in query.iter_mut() {
        // If character just started walking or is colliding, always show
        // stepping frame, and do it immediately.  Don't wait for the timer's
        // next tick.
        let is_stepping = character_option.map_or(false, |ch| {
            let state = ch.state();

            ch.is_stepping() && (ch.collision.is_solid() || state != ch.previous_state())
        });

        // Reset to the beginning of the animation when the character becomes
        // idle.
        if let Some(character) = character_option {
            if character.did_just_become_idle() {
                animated_sprite.reset();
            }
        }

        animated_sprite.timer.tick(time.delta_seconds());
        if is_stepping || animated_sprite.timer.finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).expect("should have found texture atlas handle");
            let total_num_cells = texture_atlas.textures.len();
            let (num_cells_in_animation, start_index) = match character_option {
                None => {
                    // No character.  Just use all the cells.
                    (u32::try_from(total_num_cells).expect("num cells didn't fit in u32"), 0)
                }
                Some(character) => {
                    // This animated sprite is a character.
                    let row = match character.direction {
                        Direction::North => 2,
                        Direction::South => 0,
                        Direction::East => 6,
                        Direction::West => 4,
                    };
                    let num_cells_per_row = 4;

                    match character.state() {
                        CharacterState::Idle    => (1, row * num_cells_per_row + 1),
                        CharacterState::Walking => (4, row * num_cells_per_row),
                        CharacterState::Running => (4, (row + 1) * num_cells_per_row),
                    }
                }
            };
            let mut new_anim_index = if is_stepping {
                // Index of taking a step.
                2
            } else {
                animated_sprite.animation_index + 1
            };
            new_anim_index = new_anim_index % num_cells_in_animation;
            animated_sprite.animation_index = new_anim_index;
            sprite.index = ((start_index + new_anim_index as usize) % total_num_cells) as u32;
        }
    }
}
