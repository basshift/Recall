use super::state::Difficulty;

pub const CLASSIC_LEVEL_OPTIONS: [u8; 4] = [1, 2, 3, 4];

pub fn difficulty_from_level(level: u8) -> Difficulty {
    match level {
        1 => Difficulty::Easy,
        2 => Difficulty::Medium,
        3 => Difficulty::Hard,
        _ => Difficulty::Impossible,
    }
}
