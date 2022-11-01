use bevy::prelude::*;

use crate::core::character::{Character, CharacterState, Direction};

pub(crate) fn move_sprite_system(
    time: Res<Time>,
    mut query: Query<(&Character, &mut Transform)>,
) {
    for (character, mut transform) in query.iter_mut() {
        transform.translation += character.velocity
            * time.delta_seconds()
            * character.movement_speed;
    }
}

#[derive(Debug, Component)]
pub(crate) struct AnimateTimer {
    pub timer: Timer,
}

pub(crate) fn animate_sprite_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(
        &mut AnimateTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
        &mut Character,
    )>,
) {
    for (mut timer, mut sprite, texture_atlas_handle, mut character) in
        query.iter_mut()
    {
        timer.timer.tick(time.delta());
        if timer.timer.finished() {
            let texture_atlas = texture_atlases
                .get(texture_atlas_handle)
                .expect("should have found texture atlas handle");
            let total_num_cells = texture_atlas.textures.len();
            let row = match character.direction {
                Direction::North => 3,
                Direction::South => 0,
                Direction::East => 2,
                Direction::West => 1,
            };
            let num_cells_per_row = 8;
            let (num_cells_in_animation, start_index) = match character.state {
                CharacterState::Idle => (1, row * num_cells_per_row + 1),
                CharacterState::Walking => (3, row * num_cells_per_row),
            };
            let index_in_animation =
                (character.animation_index + 1) % num_cells_in_animation;
            character.animation_index = index_in_animation;
            sprite.index =
                (start_index + index_in_animation as usize) % total_num_cells;
        }
    }
}
