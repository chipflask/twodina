use std::marker::PhantomData;
use std::convert::TryFrom;

use crate::{
    actions::DialogueActor,
    core::{
        character::{AnimatedSprite, Character, CharacterState, Direction},
        collider::{Collider, ColliderBehavior, Collision},
        config::Config,
        game::{DialogueSpec, Game},
    },
    items::ItemInteraction,
    players::Player,
};

use bevy::{ecs::component::Component, prelude::*, utils::HashSet};
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
    config: Res<Config>,
) {
    instant_move_entity(events, query, Vec3::new(2.2 * config.char_width, 0.0, 0.0));
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
// If the collision is obstructing, it stops movement
pub fn continous_move_character_system(
    time: Res<Time>,
    mut interaction_event: EventWriter<ItemInteraction>,
    mut char_query: Query<(Entity, &mut Character, Option<&mut DialogueActor>, &mut Transform, &GlobalTransform)>,
    game_state: Res<Game>,
    mut collider_query: Query<(Entity, &mut Collider, &GlobalTransform, Option<&Parent>)>,
    object_children_query: Query<&Handle<Map>>,
) {
    for (char_entity, mut character, dialogue_actor_option, mut transform, char_global) in char_query.iter_mut() {
        let char_collider = collider_query.get_component::<Collider>(char_entity).unwrap().clone();
        if character.velocity.abs_diff_eq(Vec2::ZERO, VELOCITY_EPSILON) {
            // Character has zero velocity.  Nothing to do.
            continue;
        }
        let delta: Vec2 = character.velocity * time.delta_seconds() * character.movement_speed;

        // check for collisions with objects in current map
        let char_aabb = char_collider.bounding_volume_with_translation(char_global, delta);
        let mut char_collision = Collision::empty();
        let mut dialogue_collision = None;

        for (collider_entity, collider, collider_global, maybe_parent) in collider_query.iter_mut() {
            // TODO: Use the entity instead of the map asset handle in case
            // In theory,  there can be multiple instances of the same map.

            if let Ok(owner_map) = object_children_query.get(collider_entity.clone())  {
                if *owner_map != game_state.current_map {
                    continue;
                }
            }
            if let Some(parent) = maybe_parent {
                if let Ok(owner_map) = object_children_query.get(parent.0.clone())  {
                    if *owner_map != game_state.current_map {
                        continue;
                    }
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

                    dialogue_collision = dialogue_collision.or_else(||
                        dialogue_behavior(&collision.behaviors)
                    );
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
        if let Some(mut dialogue_actor) = dialogue_actor_option {
            dialogue_actor.collider_dialogue = dialogue_collision;
        }
    }
}

fn dialogue_behavior(behaviors: &HashSet<ColliderBehavior>) -> Option<DialogueSpec> {
    for behavior in behaviors.iter() {
        match behavior {
            ColliderBehavior::Obstruct => {}
            ColliderBehavior::Collect => {}
            ColliderBehavior::Load { path: _ } => {}
            ColliderBehavior::Dialogue(spec) => {
                // If it should be auto-displayed, another system already
                // displays it.
                if !spec.auto_display {
                    return Some(spec.clone());
                }
            }
        }
    }

    None
}
