use bevy::{
    ecs::schedule::StageLabel,
    prelude::{Assets, Color, ColorMaterial, Handle, ResMut},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Loading,
    Menu,
    InGame,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Loading
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, StageLabel)]
pub enum StageLabels {
    Early,
    Later,
}

// Game state that shouldn't be saved.
#[derive(Clone, Debug)]
pub struct TransientState {
    pub debug_mode: bool,

    pub default_blue: Handle<ColorMaterial>,
    pub default_red: Handle<ColorMaterial>,
    pub button_color: Handle<ColorMaterial>,

    pub button_hovered_color: Handle<ColorMaterial>,
    pub button_pressed_color: Handle<ColorMaterial>,
}

impl TransientState {
    pub fn from_materials(materials: &mut ResMut<Assets<ColorMaterial>>, debug_mode: bool) -> TransientState {
        TransientState {
            debug_mode,

            default_blue: materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into()),
            default_red: materials.add(Color::rgba(1.0, 0.4, 0.9, 0.8).into()),
            button_color: materials.add(Color::rgb(0.4, 0.4, 0.9).into()),

            button_hovered_color: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            button_pressed_color: materials.add(Color::rgb(0.3, 0.3, 0.8).into()),
        }
    }
}
