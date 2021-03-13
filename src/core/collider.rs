use bevy::prelude::*;
use bevy::utils::HashSet;
use ncollide2d::{self as nc, bounding_volume::BoundingVolume, shape::Cuboid};

#[derive(Debug, Clone)]
pub struct Collider {
    pub behaviors: HashSet<ColliderBehavior>,
    pub shape: Cuboid<f32>,
    pub offset: Vec2,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ColliderBehavior {
    // Block movement.
    Obstruct,
    // Picked up by character.
    PickUp,
    // Collected by character.
    Collect,
    // open a new level
    Load { path: String },
    // Begin dialogue.
    Dialogue(String),
}

#[derive(Clone, Debug, Default)]
pub struct Collision {
    pub behaviors: HashSet<ColliderBehavior>,
}

impl Collider {
    pub fn new(behaviors: HashSet<ColliderBehavior>, width_height: Vec2, offset: Vec2) -> Collider {
        let half_extent = width_height / 2.0;
        let v2 = nc::math::Vector::new(half_extent.x, half_extent.y);
        Collider {
            behaviors,
            shape: Cuboid::new(v2),
            offset,
        }
    }

    pub fn single(behavior: ColliderBehavior, width_height: Vec2, offset: Vec2) -> Collider {
        let mut behaviors: HashSet<ColliderBehavior> = HashSet::default();
        behaviors.insert(behavior);

        Self::new(behaviors, width_height, offset)
    }

    pub fn insert_behavior(&mut self, behavior: ColliderBehavior) {
        self.behaviors.insert(behavior);
    }

    pub fn remove_behavior(&mut self, behavior: &ColliderBehavior) {
        self.behaviors.remove(behavior);
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

    pub fn intersect(&self, global_transform: &GlobalTransform, other: &nc::bounding_volume::AABB<f32>) -> Option<Collision> {
        if self.behaviors.is_empty() {
            return None;
        }

        let aabb = self.bounding_volume(global_transform);

        if !aabb.intersects(other) {
            return None;
        }

        Some(Collision {
            behaviors: self.behaviors.clone(),
        })
    }
}

impl Collision {
    pub fn empty() -> Collision {
        Collision {
            behaviors: HashSet::default(),
        }
    }

    pub fn insert_behavior(&mut self, behavior: ColliderBehavior) {
        self.behaviors.insert(behavior);
    }
    
    pub fn is_obstruction(&self) -> bool {
        for behavior in self.behaviors.iter() {
            match behavior {
                ColliderBehavior::Obstruct => return true,
                ColliderBehavior::PickUp |
                ColliderBehavior::Collect |
                ColliderBehavior::Load { path: _ } |
                ColliderBehavior::Dialogue(_) => {}
            }
        }

        false
    }
}
