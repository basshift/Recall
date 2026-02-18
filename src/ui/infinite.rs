use super::state::{AppState, Difficulty};

pub const START_LEVEL: u8 = 1;
const EASY_END_ROUND: u32 = 3;
const NORMAL_END_ROUND: u32 = 6;
const HARD_END_ROUND: u32 = 10;

#[derive(Clone, Copy, Debug)]
pub struct LevelUpEvent {
    pub from_level: u8,
    pub to_level: u8,
    pub round: u32,
}

pub fn mode_label(st: &AppState) -> String {
    if classic_difficulty_for_round(st.infinite_round) == Difficulty::Impossible {
        format!(
            "Infinite Expert Survival {}",
            expert_survival_rounds(st.infinite_round)
        )
    } else if st.recall_level >= 3 {
        format!(
            "Infinite Hard Survival {}",
            hard_survival_rounds(st.infinite_round)
        )
    } else {
        format!("Infinite Round {}", st.infinite_round)
    }
}

pub fn is_infinite(difficulty: Difficulty) -> bool {
    difficulty == Difficulty::RecallMode
}

pub fn prepare_start(st: &mut AppState) {
    st.apply_infinite_level_without_reset(START_LEVEL);
    st.reset_infinite_round();
}

pub fn level_name(level: u8) -> &'static str {
    match level.clamp(1, 4) {
        1 => "Easy",
        2 => "Normal",
        3 => "Hard",
        _ => "Expert",
    }
}

pub fn level_for_round(round: u32) -> u8 {
    if round <= EASY_END_ROUND {
        1
    } else if round <= NORMAL_END_ROUND {
        2
    } else if round <= HARD_END_ROUND {
        3
    } else {
        4
    }
}

pub fn projected_level_for_next_round(st: &AppState) -> u8 {
    level_for_round(st.infinite_round.saturating_add(1))
}

pub fn hard_survival_rounds(round: u32) -> u32 {
    round.saturating_sub(NORMAL_END_ROUND)
}

pub fn expert_survival_rounds(round: u32) -> u32 {
    round.saturating_sub(HARD_END_ROUND)
}

pub fn classic_difficulty_for_round(round: u32) -> Difficulty {
    if round <= EASY_END_ROUND {
        Difficulty::Easy
    } else if round <= NORMAL_END_ROUND {
        Difficulty::Medium
    } else if round <= HARD_END_ROUND {
        Difficulty::Hard
    } else {
        Difficulty::Impossible
    }
}

pub fn advance_round(st: &mut AppState) -> Option<LevelUpEvent> {
    if !is_infinite(st.difficulty) {
        return None;
    }

    let previous_classic_difficulty = classic_difficulty_for_round(st.infinite_round);
    let previous_level = st.recall_level;
    st.advance_infinite_round();
    let next_classic_difficulty = classic_difficulty_for_round(st.infinite_round);
    if next_classic_difficulty != previous_classic_difficulty {
        st.reset_impossible_pressure();
    }
    let target_level = level_for_round(st.infinite_round);
    if target_level != previous_level {
        st.apply_infinite_level_without_reset(target_level);
        return Some(LevelUpEvent {
            from_level: previous_level,
            to_level: target_level,
            round: st.infinite_round,
        });
    }

    None
}
