use bevy::math::Vec2;
use ncollide2d::{math, shape::Cuboid};

#[derive(Debug)]
pub struct Collider {
    pub bounding_volume: Cuboid<f32>,
}

impl Collider {
    pub fn new(width_height: Vec2) -> Collider {
        let half_extent = width_height;
        let v2 = math::Vector::new(half_extent.x, half_extent.y);
        Collider {
            bounding_volume: Cuboid::new(v2),
        }
    }
}
