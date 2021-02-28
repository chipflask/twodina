use bevy::prelude::*;

use crate::character::Character;
use crate::items::Inventory;
use crate::players::Player;

// Debug entities will be marked with this so that we can despawn them all when
// debug mode is turned off.
#[derive(Debug, Default)]
pub struct Debuggable;

pub struct PlayerPositionDisplay {
    pub player_id: u32,
}


pub fn position_display_system(
    mut character_query: Query<(&Transform, &Player, &Character, &Inventory)>,
    mut text_query: Query<(&mut Text, &PlayerPositionDisplay)>,
) {
    for (char_transform, player, character, inventory) in character_query.iter_mut() {
        for (mut text, ppd) in text_query.iter_mut() {
            if ppd.player_id == player.id {
                text.sections[0].value = format!(
                    "P{} Position: ({:.1}, {:.1}, {:.1}) collision={:?} gems={:?}",
                    player.id + 1,
                    char_transform.translation.x,
                    char_transform.translation.y,
                    char_transform.translation.z,
                    character.collision,
                    inventory.num_gems
                );
            }
        }
    }
}
