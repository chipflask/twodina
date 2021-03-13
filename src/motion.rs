use std::marker::PhantomData;
use std::convert::TryFrom;

use crate::{
    core::{
        character::{AnimatedSprite, Character, CharacterState, Direction},
        collider::{Collider, Collision},
        game::Game,
    },
    players::{Player, PLAYER_WIDTH},
    items::ItemInteraction,
};

use bevy::{
    prelude::*,
    ecs::component::Component,
};
use bevy_tiled_prototype::Map;


// If two scalars have an absolute value difference less than this, then they're
// considered equal.
pub const VELOCITY_EPSILON: f32 = 0.001;

// todo: utils.rs
// Return the Z translation for a given Y translation.  Z determines occlusion.
pub fn z_from_y(y: f32) -> f32 {
    -y / 100.0
}

pub struct MoveEntityEvent<T: Component> {
    pub object_component: PhantomData<T>,
    pub target: Entity,
}

pub fn instant_move_player_system(
    events: EventReader<MoveEntityEvent<Player>>,
    query: Query<(&mut Transform, Option<&Player>)>,
) {
    instant_move_entity(events, query, Vec3::new(2.2 * PLAYER_WIDTH, 0.0, 0.0));
}

// Currently used for warping between levels, but could be useful for many other things
fn instant_move_entity<T: Component>(
    mut events: EventReader<MoveEntityEvent<T>>,
    mut query: Query<(&mut Transform, Option<&T>)>,
    offset: Vec3, // additive
) {
    for event in events.iter() {
        let target = match query.get_mut(event.target) {
            Ok((transform, _)) => transform.translation,
            Err(_) => continue,
        };
        let mut total_offset = Vec3::ZERO;
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

            ch.is_stepping() && (ch.collision.is_obstruction() || state != ch.previous_state())
        });

        // Reset to the beginning of the animation when the character becomes
        // idle.
        if let Some(character) = character_option {
            if character.did_just_become_idle() {
                animated_sprite.reset();
            }
        }

        animated_sprite.timer.tick(time.delta());
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

// This system applies a velocity and checks for collisions.
// If the collision is occluding, it stops movement
pub fn continous_move_character_system(
    time: Res<Time>,
    mut interaction_event: EventWriter<ItemInteraction>,
    mut char_query: Query<(Entity, &mut Character, &mut Transform, &GlobalTransform)>,
    game_state: Res<Game>,
    mut collider_query: Query<(Entity, &mut Collider, &GlobalTransform, Option<&Handle<Map>>)>,
) {
    for (char_entity, mut character, mut transform, char_global) in char_query.iter_mut() {
        let char_collider = collider_query.get_component::<Collider>(char_entity).unwrap().clone();
        if character.velocity.abs_diff_eq(Vec2::ZERO, VELOCITY_EPSILON) {
            // Character has zero velocity.  Nothing to do.
            continue;
        }
        let delta: Vec2 = character.velocity * time.delta_seconds() * character.movement_speed;

        // check for collisions with objects in current map
        let char_aabb = char_collider.bounding_volume_with_translation(char_global, delta);
        let mut char_collision = Collision::empty();
        for (collider_entity, collider, collider_global, option_to_map) in collider_query.iter_mut() {
            // TODO: Use the entity instead of the map asset handle in case
            // In theory,  there can be multiple instances of the same map.
            if let Some(owner_map) = option_to_map  {
                if *owner_map != game_state.current_map {
                    continue;
                }
            }
            // Shouldn't collide with itself.
            if collider_entity == char_entity {
                continue;
            }
            match collider.intersect(collider_global, &char_aabb) {
                None => {}
                Some(collision) => {
                    for behavior in collision.behaviors.iter() {
                        char_collision.insert_behavior(behavior.clone());
                    }

                    interaction_event.send(ItemInteraction::new(
                        char_entity,
                        collider_entity,
                        collision.behaviors,
                    ));
                }
            }
        }
        if !char_collision.is_obstruction() {
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
            // Z needs to reflect where the character is on the ground, and
            // presumably, that's where the character collides.  So we add the
            // collider's Z offset to the translation.
            transform.translation.z = z_from_y(transform.translation.y + char_collider.offset.y);
        }
        character.collision = char_collision;
    }
}
