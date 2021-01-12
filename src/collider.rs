use bevy::math::Vec2;
use ncollide2d::{math, shape::Cuboid};

#[derive(Debug)]
pub struct Collider {
    pub shape: Cuboid<f32>,
    pub offset: Vec2,
}

impl Collider {
    pub fn new(width_height: Vec2, offset: Vec2) -> Collider {
        let half_extent = width_height / 2.0;
        let v2 = math::Vector::new(half_extent.x, half_extent.y);
        Collider {
            shape: Cuboid::new(v2),
            offset: offset
        }
    }
}
