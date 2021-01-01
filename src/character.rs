#[derive(Debug)]
pub struct Character {
    pub direction: Direction,
    pub state: CharacterState,
    pub animation_index: u32,
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
        self.animation_index = 0;
    }
}

impl Default for Character {
    fn default() -> Character {
        Character {
            direction: Direction::South,
            state: CharacterState::Idle,
            animation_index: 0,
        }
    }
}
