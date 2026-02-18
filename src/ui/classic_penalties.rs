use super::state::{AppState, Difficulty};

pub const MISMATCH_THRESHOLD: u8 = 3;
pub const PREVIEW_SECONDS: f64 = 1.4;
const MEDIUM_MISMATCH_THRESHOLD: u8 = 5;
const HARD_MISMATCH_THRESHOLD: u8 = 2;
const HARD_RESHUFFLE_REVEAL_MS: u64 = 950;

#[derive(Clone, Copy, Debug)]
pub struct PunishmentPlan {
    pub reveal_count: usize,
    pub reveal_ms: u64,
    pub reshuffle_hidden: bool,
    pub reveal_all_hidden: bool,
    pub source_difficulty: Difficulty,
}

pub fn is_expert(difficulty: Difficulty) -> bool {
    difficulty == Difficulty::Impossible
}

pub fn mismatch_pause_ms(difficulty: Difficulty) -> u64 {
    match difficulty {
        Difficulty::Easy => 750,
        _ => 500,
    }
}

pub fn register_mismatch_and_plan_reshuffle_for(
    st: &mut AppState,
    first_pick_index: usize,
    difficulty: Difficulty,
) -> Option<PunishmentPlan> {
    match difficulty {
        Difficulty::Easy => return None,
        Difficulty::Medium => {
            st.impossible_mismatch_count = st.impossible_mismatch_count.saturating_add(1);
            if st.impossible_mismatch_count < MEDIUM_MISMATCH_THRESHOLD {
                return None;
            }
            st.reset_impossible_pressure();
            return Some(PunishmentPlan {
                reveal_count: 2,
                reveal_ms: 320,
                reshuffle_hidden: true,
                reveal_all_hidden: false,
                source_difficulty: difficulty,
            });
        }
        Difficulty::Hard => {
            st.impossible_mismatch_count = st.impossible_mismatch_count.saturating_add(1);
            if st.impossible_mismatch_count < HARD_MISMATCH_THRESHOLD {
                return None;
            }
            st.reset_impossible_pressure();
            return Some(PunishmentPlan {
                reveal_count: 0,
                reveal_ms: HARD_RESHUFFLE_REVEAL_MS,
                reshuffle_hidden: true,
                reveal_all_hidden: true,
                source_difficulty: difficulty,
            });
        }
        Difficulty::Impossible => {}
        _ => return None,
    }

    if st.impossible_last_first_index == Some(first_pick_index) {
        st.impossible_same_first_streak = st.impossible_same_first_streak.saturating_add(1);
    } else {
        st.impossible_last_first_index = Some(first_pick_index);
        st.impossible_same_first_streak = 1;
    }

    st.impossible_mismatch_count = st.impossible_mismatch_count.saturating_add(1);
    let threshold_hit = st.impossible_mismatch_count >= MISMATCH_THRESHOLD;
    let repeated_first_hit = st.impossible_same_first_streak >= 2;
    let should_punish = threshold_hit || repeated_first_hit;

    if !should_punish {
        return None;
    }

    st.impossible_mismatch_count = 0;
    st.impossible_same_first_streak = 0;
    st.impossible_last_first_index = None;
    st.impossible_punish_stage = st.impossible_punish_stage.saturating_add(1);

    let (reveal_count, reveal_ms) = match st.impossible_punish_stage {
        1 => (7, 650),
        2 => (5, 540),
        _ => (4, 430),
    };

    Some(PunishmentPlan {
        reveal_count,
        reveal_ms,
        reshuffle_hidden: true,
        reveal_all_hidden: false,
        source_difficulty: difficulty,
    })
}

pub fn reset_penalty_after_match_for(st: &mut AppState, difficulty: Difficulty) {
    if matches!(difficulty, Difficulty::Medium | Difficulty::Hard | Difficulty::Impossible) {
        st.reset_impossible_pressure();
    }
}
