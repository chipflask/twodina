use bevy::prelude::*;

use crate::collider::{Collider, ColliderType};

#[derive(Debug)]
pub struct PickUpEvent {
    actor: Entity,
    object: Entity,
}

impl PickUpEvent {
    pub fn new(actor: Entity, object: Entity) -> PickUpEvent {
        // An entity can't pick up itself.
        assert!(actor != object);

        PickUpEvent {
            actor,
            object,
        }
    }
}

pub fn pick_up_system(
    commands: &mut Commands,
    mut pick_up_event_reader: Local<EventReader<PickUpEvent>>,
    pick_up_events: Res<Events<PickUpEvent>>,
    mut query: Query<(&mut Transform, Option<&mut Collider>)>,
) {
    for pick_up_event in pick_up_event_reader.iter(&pick_up_events) {
        let actor_scale = match query.get_mut(pick_up_event.actor) {
            Ok((actor_transform, _)) => actor_transform.scale.clone(),
            Err(_) => continue,
        };
        if let Ok((mut object_transform, object_collider_option)) = query.get_mut(pick_up_event.object) {
            // TODO: This value is hardcoded for the shield.
            object_transform.translation = Vec3::new(0.0, -10.0, 0.0);
            object_transform.scale /= actor_scale;
            // If the object has a Collider component, stop colliding so that it
            // doesn't get picked up again.
            if let Some(mut object_collider) = object_collider_option {
                object_collider.collider_type = ColliderType::Ignore;
            }
            commands.push_children(pick_up_event.actor, &[pick_up_event.object]);
        }
    }
}
