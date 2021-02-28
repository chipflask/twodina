#[derive(Debug, Copy, Clone)]
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

// run loop stages
pub const EARLY: &str = "EARLY";
pub const LATER: &str = "LATER";
