use bevy::prelude::*;
use ncollide2d::{self as nc, bounding_volume::BoundingVolume, shape::Cuboid};

#[derive(Debug)]
pub struct Collider {
    pub collider_type: ColliderType,
    pub shape: Cuboid<f32>,
    pub offset: Vec2,
}

#[derive(Copy, Clone, Debug)]
pub enum ColliderType {
    Solid,
    PickUp,
}

#[derive(Copy, Clone, Debug)]
pub enum Collision {
    NoCollision,
    Solid,
    PickUp,
}

impl Collider {
    pub fn new(collider_type: ColliderType, width_height: Vec2, offset: Vec2) -> Collider {
        let half_extent = width_height / 2.0;
        let v2 = nc::math::Vector::new(half_extent.x, half_extent.y);
        Collider {
            collider_type,
            shape: Cuboid::new(v2),
            offset: offset
        }
    }

    pub fn bounding_volume(&self, global_trans: &GlobalTransform) -> nc::bounding_volume::AABB<f32> {
        self.bounding_volume_with_translation(global_trans, Vec2::zero())
    }

    pub fn bounding_volume_with_translation(&self,
        global_trans: &GlobalTransform,
        delta: Vec2,
    ) -> nc::bounding_volume::AABB<f32> {

        // TODO: Handle scale and rotation.
        let isometry = nc::math::Isometry::translation(
            global_trans.translation.x + delta.x + self.offset.x,
            global_trans.translation.y + delta.y + self.offset.y);

        nc::bounding_volume::aabb(&self.shape, &isometry)
    }

    pub fn intersect(&self, global_transform: &GlobalTransform, other: &nc::bounding_volume::AABB<f32>) -> Collision {
        let aabb = self.bounding_volume(global_transform);

        if !aabb.intersects(other) {
            return Collision::NoCollision;
        }

        match self.collider_type {
            ColliderType::Solid => Collision::Solid,
            ColliderType::PickUp => Collision::PickUp,
        }
    }
}

impl Collision {
    pub fn is_solid(&self) -> bool {
        match self {
            Collision::Solid => true,
            Collision::NoCollision | Collision::PickUp => false,
        }
    }
}
