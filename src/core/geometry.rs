use bevy::reflect::Reflect;

// This is a reproduction of Rect that was in older versions of Bevy.
//
// TODO: Use bevy::sprite::Rect which is going to get promoted to bevy_math.
// See https://github.com/bevyengine/bevy/issues/5575.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}
