use bevy::math::Vec3;
use bevy::prelude::Timer;

#[derive(Debug)]
pub struct Character {
    pub direction: Direction,
    pub state: CharacterState,
    pub velocity: Vec3,
    pub movement_speed: f32,
}

#[derive(Debug)]
pub struct AnimatedSprite {
    pub animation_index: u32,
    pub timer: Timer,
}

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Copy, Clone, Debug)]
pub enum CharacterState {
    Idle,
    Walking,
}

impl Character {
    pub fn make_idle(&mut self) {
        self.state = CharacterState::Idle;
    }
}

impl Default for Character {
    fn default() -> Self {
        Character {
            direction: Direction::South,
            state: CharacterState::Idle,
            velocity: Vec3::zero(),
            movement_speed: 175.0,
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
