use bevy::math::Vec2;
use bevy::prelude::Timer;

pub const WALK_SPEED: f32 = 175.0;
pub const RUN_SPEED: f32 = 400.0;

// If two scalars have an absolute value difference less than this, then they're
// considered equal.
pub const VELOCITY_EPSILON: f32 = 0.001;

#[derive(Debug)]
pub struct Character {
    pub direction: Direction,
    state: CharacterState,
    previous_state: CharacterState,
    pub velocity: Vec2,
    pub movement_speed: f32,
    pub is_colliding: bool,
}

#[derive(Debug)]
pub struct AnimatedSprite {
    pub animation_index: u32,
    pub timer: Timer,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CharacterState {
    Idle,
    Walking,
}

impl Character {
    pub fn state(&self) -> CharacterState {
        self.state
    }

    pub fn set_state(&mut self, state: CharacterState) {
        self.previous_state = self.state;
        self.state = state;
    }

    pub fn previous_state(&self) -> CharacterState {
        self.previous_state
    }

    pub fn did_just_become_idle(&self) -> bool {
        self.previous_state != self.state && self.state == CharacterState::Idle
    }
}

impl Default for Character {
    fn default() -> Self {
        Character {
            direction: Direction::South,
            state: CharacterState::Idle,
            previous_state: CharacterState::Idle,
            velocity: Vec2::zero(),
            movement_speed: WALK_SPEED,
            is_colliding: false,
        }
    }
}

impl AnimatedSprite {
    // Specify the amount of time for each animation frame in seconds.
    pub fn with_frame_seconds(seconds: f32) -> AnimatedSprite {
        AnimatedSprite {
            animation_index: 0,
            timer: Timer::from_seconds(seconds, true),
        }
    }

    pub fn reset(&mut self) {
        self.animation_index = 0;
    }
}
