use bevy::prelude::*;

use crate::{AppState, LATER, collider::{Collider, ColliderBehavior}};

#[derive(Debug, Default)]
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<Interaction>()
            .on_state_update(LATER, AppState::InGame, items_system.system());
    }
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub num_gems: u32,
}

// Event to specify that an actor should pick up an item and equip it.
#[derive(Debug)]
pub struct Interaction {
    actor: Entity,
    object: Entity,
    behavior: ColliderBehavior,
}

// Transform to apply to an item when it's equipped.
#[derive(Debug)]
pub struct EquippedTransform {
    pub transform: Transform,
}

impl Interaction {
    pub fn new(actor: Entity, object: Entity, behavior: ColliderBehavior) -> Interaction {
        // An entity can't pick up itself.
        assert!(actor != object);

        Interaction {
            actor,
            object,
            behavior,
        }
    }
}

// handles consume and equip
pub fn items_system(
    commands: &mut Commands,
    mut interaction_reader: Local<EventReader<Interaction>>,
    interactions: Res<Events<Interaction>>,
    mut query: Query<(&mut Transform, Option<&EquippedTransform>, Option<&mut Collider>)>,
    mut inventory_query: Query<&mut Inventory>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    for interaction in interaction_reader.iter(&interactions) {
        match interaction.behavior {
            ColliderBehavior::Collect => {
                commands.despawn_recursive(interaction.object);
                if let Ok(mut inventory) = inventory_query.get_mut(interaction.actor) {
                    inventory.num_gems += 1;
                    // should be more parametric
                    let music = asset_server.load("sfx/gem_small.ogg");
                    audio.play(music);
                }
                continue;
            }
            _ => (),
        }
        let actor_scale = match query.get_mut(interaction.actor) {
            Ok((actor_transform, _, _)) => actor_transform.scale.clone(),
            Err(_) => continue,
        };
        if let Ok((mut object_transform, equipped_transform_option, object_collider_option)) = query.get_mut(interaction.object) {
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
                object_collider.behavior = ColliderBehavior::Ignore;
            }
            commands.push_children(interaction.actor, &[interaction.object]);
        }
    }
}
