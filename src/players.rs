use bevy::prelude::Component;

#[derive(Debug, Component)]
pub struct Player {
    pub width: f32,
    pub height: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            width: 31.0,
            height: 32.0,
        }
    }
}
