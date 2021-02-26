use std::marker::PhantomData;

use crate::{Player, PLAYER_WIDTH};
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
