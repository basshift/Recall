use super::classic_penalties::PunishmentPlan;
use super::state::{AppState, Difficulty, TileStatus};

const TRI_NORMAL_MISMATCH_THRESHOLD: u8 = 5;
const TRI_HARD_MISMATCH_THRESHOLD: u8 = 3;
const TRI_EXPERT_MISMATCH_THRESHOLD: u8 = 3;

pub fn mismatch_pause_ms(level: u8) -> u64 {
    match level.clamp(1, 4) {
        1 => 800,
        2 => 650,
        3 => 550,
        _ => 500,
    }
}

pub fn register_mismatch_and_plan_reshuffle(
    st: &mut AppState,
    first_pick_index: usize,
) -> Option<PunishmentPlan> {
    match st.tri_level.clamp(1, 4) {
        1 => return None,
        2 => {
            st.impossible_mismatch_count = st.impossible_mismatch_count.saturating_add(1);
            if st.impossible_mismatch_count < TRI_NORMAL_MISMATCH_THRESHOLD {
                return None;
            }
            st.reset_impossible_pressure();
            return Some(PunishmentPlan {
                reveal_count: 3,
                reveal_ms: 340,
                reshuffle_hidden: true,
                reveal_all_hidden: false,
                source_difficulty: Difficulty::Tri,
            });
        }
        3 => {
            st.impossible_mismatch_count = st.impossible_mismatch_count.saturating_add(1);
            if st.impossible_mismatch_count < TRI_HARD_MISMATCH_THRESHOLD {
                return None;
            }
            st.reset_impossible_pressure();
            return Some(PunishmentPlan {
                reveal_count: 0,
                reveal_ms: 950,
                reshuffle_hidden: true,
                reveal_all_hidden: true,
                source_difficulty: Difficulty::Tri,
            });
        }
        _ => {}
    }

    if st.impossible_last_first_index == Some(first_pick_index) {
        st.impossible_same_first_streak = st.impossible_same_first_streak.saturating_add(1);
    } else {
        st.impossible_last_first_index = Some(first_pick_index);
        st.impossible_same_first_streak = 1;
    }

    st.impossible_mismatch_count = st.impossible_mismatch_count.saturating_add(1);
    let threshold_hit = st.impossible_mismatch_count >= TRI_EXPERT_MISMATCH_THRESHOLD;
    let repeated_first_hit = st.impossible_same_first_streak >= 2;
    let should_punish = threshold_hit || repeated_first_hit;

    if !should_punish {
        return None;
    }

    st.impossible_mismatch_count = 0;
    st.impossible_same_first_streak = 0;
    st.impossible_last_first_index = None;
    st.impossible_punish_stage = st.impossible_punish_stage.saturating_add(1);

    let hidden_count = st
        .tiles
        .iter()
        .filter(|tile| tile.status == TileStatus::Hidden)
        .count();
    let (base_reveal_count, reveal_ms) = match st.impossible_punish_stage {
        1 => (9, 700),
        2 => (7, 590),
        _ => (5, 480),
    };
    let reveal_count = base_reveal_count.min(hidden_count);

    Some(PunishmentPlan {
        reveal_count,
        reveal_ms,
        reshuffle_hidden: true,
        reveal_all_hidden: false,
        source_difficulty: Difficulty::Tri,
    })
}

pub fn reset_penalty_after_match(st: &mut AppState) {
    if st.tri_level < 2 {
        return;
    }
    st.reset_impossible_pressure();
}
