use bevy::prelude::*;
use ncollide2d::{self as nc, bounding_volume::BoundingVolume, shape::Cuboid};

#[derive(Debug, Clone)]
pub struct Collider {
    pub behavior: ColliderBehavior,
    pub shape: Cuboid<f32>,
    pub offset: Vec2,
}

#[derive(Clone, Debug)]
pub enum ColliderBehavior {
    // Block movement.
    Obstruct,
    // Picked up by character.
    PickUp,
    // Collected by character.
    Collect,
    // open a new level
    Load { path: String },
    // Hit test is skipped.
    Ignore,
}

#[derive(Clone, Debug)]
pub enum Collision {
    Nil,
    Obstruction,
    Interaction(ColliderBehavior),
}

impl Collider {
    pub fn new(collider_type: ColliderBehavior, width_height: Vec2, offset: Vec2) -> Collider {
        let half_extent = width_height / 2.0;
        let v2 = nc::math::Vector::new(half_extent.x, half_extent.y);
        Collider {
            behavior: collider_type,
            shape: Cuboid::new(v2),
            offset: offset
        }
    }

    pub fn bounding_volume(&self, global_trans: &GlobalTransform) -> nc::bounding_volume::AABB<f32> {
        self.bounding_volume_with_translation(global_trans, Vec2::ZERO)
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
        match &self.behavior {
            ColliderBehavior::Obstruct | ColliderBehavior::PickUp | ColliderBehavior::Collect | ColliderBehavior::Load { path: _ } => (),
            ColliderBehavior::Ignore => return Collision::Nil,
        }

        let aabb = self.bounding_volume(global_transform);

        if !aabb.intersects(other) {
            return Collision::Nil;
        }

        match self.behavior {
            ColliderBehavior::Obstruct => Collision::Obstruction,
            ColliderBehavior::PickUp | ColliderBehavior::Collect | ColliderBehavior::Load { path: _ }
                => Collision::Interaction(self.behavior.clone()),
            ColliderBehavior::Ignore => panic!("Should never reach here"),
        }
    }
}

impl Collision {
    pub fn is_solid(&self) -> bool {
        match self {
            Collision::Obstruction => true,
            Collision::Nil | Collision::Interaction(_) => false,
        }
    }
}
