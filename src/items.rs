use bevy::prelude::*;

use crate::collider::{Collider, ColliderType};

#[derive(Debug, Default)]
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<PickUpEvent>()
            .add_system(pick_up_system.system());
    }
}

// Event to specify that an actor should pick up an item and equip it.
#[derive(Debug)]
pub struct PickUpEvent {
    actor: Entity,
    object: Entity,
}

// Transform to apply to an item when it's equipped.
#[derive(Debug)]
pub struct EquippedTransform {
    pub transform: Transform,
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
    mut query: Query<(&mut Transform, Option<&EquippedTransform>, Option<&mut Collider>)>,
) {
    for pick_up_event in pick_up_event_reader.iter(&pick_up_events) {
        let actor_scale = match query.get_mut(pick_up_event.actor) {
            Ok((actor_transform, _, _)) => actor_transform.scale.clone(),
            Err(_) => continue,
        };
        if let Ok((mut object_transform, equipped_transform_option, object_collider_option)) = query.get_mut(pick_up_event.object) {
            // An object can have a special transform applied when equipped.
            if let Some(equipped) = equipped_transform_option {
                object_transform.translation = equipped.transform.translation;
                object_transform.rotation = equipped.transform.rotation;
                object_transform.scale = equipped.transform.scale;
            }
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
