use super::state::Difficulty;

pub const CLASSIC_LEVEL_OPTIONS: [(&str, u8); 4] =
    [("Easy", 1), ("Normal", 2), ("Hard", 3), ("Expert", 4)];

pub fn difficulty_from_level(level: u8) -> Difficulty {
    match level {
        1 => Difficulty::Easy,
        2 => Difficulty::Medium,
        3 => Difficulty::Hard,
        _ => Difficulty::Impossible,
    }
}
