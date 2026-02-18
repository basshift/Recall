use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;

use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use gio::SimpleAction;

use super::board::CONTENT_MARGIN;
use super::dialogs::{show_about_dialog, show_instructions_dialog};
use super::hud::{
    set_header_game,
    set_header_menu,
    start_preview_phase,
    start_timer,
    stop_preview,
    stop_timer,
    update_subtitle,
};
use super::infinite;
use super::classic_penalties;
use super::mode_dialogs::show_mode_dialog;
use super::records::{
    load_records,
    register_infinite_round_result,
    register_non_infinite_result,
    show_memory_dialog,
};
use super::scene::{build_board_for_difficulty, rebuild_board, show_menu, show_victory};
use super::session_save;
use super::state::{AppState, Difficulty, TileStatus};
use super::tri_penalties;
use super::debug_tools;
use super::infinite_flow;

pub(super) fn clear_flip_classes(button: &gtk::Button) {
    button.remove_css_class("flip-hide");
    button.remove_css_class("flip-show");
    button.remove_css_class("flip-show-a");
    button.remove_css_class("flip-show-b");
    button.remove_css_class("reshuffle-flip");
    button.remove_css_class("hard-reshuffle-fast");
    button.remove_css_class("infinite-round-flip");
}

pub(super) fn redraw_button_child(button: &gtk::Button) {
    if let Some(child) = button.child() {
        child.queue_draw();
    }
}

pub(super) fn play_flip_show(st: &mut AppState, index: usize) {
    let button = st.grid_buttons[index].clone();
    clear_flip_classes(&button);
    st.flip_anim_phase = !st.flip_anim_phase;
    if st.flip_anim_phase {
        button.add_css_class("flip-show-a");
    } else {
        button.add_css_class("flip-show-b");
    }
    redraw_button_child(&button);
}

enum FlipOutcome {
    Continue,
    Mismatch,
    CompleteMatch,
}

fn evaluate_flip_outcome(st: &AppState, indices: &[usize], latest_index: usize) -> FlipOutcome {
    if indices.len() > 1 {
        let first_value = &st.tiles[indices[0]].value;
        let current_value = &st.tiles[latest_index].value;
        if current_value != first_value {
            return FlipOutcome::Mismatch;
        }
    }

    if indices.len() == st.match_size {
        FlipOutcome::CompleteMatch
    } else {
        FlipOutcome::Continue
    }
}

const FLIP_PHASE_MS: u64 = 260;
const CLASSIC_RESHUFFLE_FLIP_MS: u64 = 760;
const HARD_ENDGAME_RESHUFFLE_FLIP_MS: u64 = 620;
const INFINITE_PRE_TRANSITION_WAIT_MS: u64 = 500;
const MATCH_BUMP_DELAY_MS: u64 = 250;
const MATCH_BUMP_DURATION_MS: u64 = 1300;
const PREVIEW_REVEAL_MIN_DELAY_MS: u64 = 500;
#[derive(Clone, Copy)]
struct CascadeProfile {
    start_delay_ms: u64,
    base_step_ms: u64,
    base_pause_ms: u64,
    step_min_ms: u64,
    step_max_ms: u64,
    pause_min_ms: u64,
    pause_max_ms: u64,
    dual_corner_wave: bool,
}

fn cascade_profile_for(st: &AppState) -> CascadeProfile {
    match st.difficulty {
        Difficulty::Easy => CascadeProfile {
            start_delay_ms: 700,
            base_step_ms: 150,
            base_pause_ms: 100,
            step_min_ms: 80,
            step_max_ms: 260,
            pause_min_ms: 80,
            pause_max_ms: 220,
            dual_corner_wave: false,
        },
        Difficulty::Medium => CascadeProfile {
            start_delay_ms: 620,
            base_step_ms: 132,
            base_pause_ms: 88,
            step_min_ms: 74,
            step_max_ms: 220,
            pause_min_ms: 74,
            pause_max_ms: 185,
            dual_corner_wave: false,
        },
        Difficulty::Hard => CascadeProfile {
            start_delay_ms: 460,
            base_step_ms: 108,
            base_pause_ms: 70,
            step_min_ms: 60,
            step_max_ms: 172,
            pause_min_ms: 54,
            pause_max_ms: 138,
            dual_corner_wave: true,
        },
        Difficulty::Impossible => CascadeProfile {
            start_delay_ms: 390,
            base_step_ms: 96,
            base_pause_ms: 61,
            step_min_ms: 54,
            step_max_ms: 158,
            pause_min_ms: 50,
            pause_max_ms: 124,
            dual_corner_wave: true,
        },
        Difficulty::Tri => match st.tri_level.clamp(1, 4) {
            1 => CascadeProfile {
                start_delay_ms: 650,
                base_step_ms: 142,
                base_pause_ms: 94,
                step_min_ms: 78,
                step_max_ms: 240,
                pause_min_ms: 78,
                pause_max_ms: 205,
                dual_corner_wave: false,
            },
            2 => CascadeProfile {
                start_delay_ms: 560,
                base_step_ms: 122,
                base_pause_ms: 80,
                step_min_ms: 68,
                step_max_ms: 196,
                pause_min_ms: 64,
                pause_max_ms: 162,
                dual_corner_wave: false,
            },
            3 => CascadeProfile {
                start_delay_ms: 460,
                base_step_ms: 106,
                base_pause_ms: 68,
                step_min_ms: 58,
                step_max_ms: 168,
                pause_min_ms: 52,
                pause_max_ms: 134,
                dual_corner_wave: true,
            },
            _ => CascadeProfile {
                start_delay_ms: 400,
                base_step_ms: 94,
                base_pause_ms: 60,
                step_min_ms: 54,
                step_max_ms: 156,
                pause_min_ms: 50,
                pause_max_ms: 122,
                dual_corner_wave: true,
            },
        },
        _ => CascadeProfile {
            start_delay_ms: 640,
            base_step_ms: 138,
            base_pause_ms: 92,
            step_min_ms: 76,
            step_max_ms: 230,
            pause_min_ms: 76,
            pause_max_ms: 195,
            dual_corner_wave: false,
        },
    }
}

fn victory_cascade_start_delay_ms(st: &AppState) -> u64 {
    cascade_profile_for(st).start_delay_ms
}

fn balanced_cascade_timings(total_cards: usize, profile: CascadeProfile) -> (u64, u64) {
    let normalized = (total_cards.max(1) as f64) / 12.0;
    let scale = normalized.sqrt();
    let step_ms = (profile.base_step_ms as f64 * scale).round() as u64;
    let pause_ms = (profile.base_pause_ms as f64 * scale).round() as u64;

    (
        step_ms.clamp(profile.step_min_ms, profile.step_max_ms),
        pause_ms.clamp(profile.pause_min_ms, profile.pause_max_ms),
    )
}

fn build_cascade_waves(total_cards: usize, dual_corner_wave: bool) -> Vec<Vec<usize>> {
    if total_cards == 0 {
        return Vec::new();
    }
    if !dual_corner_wave {
        return (0..total_cards).map(|idx| vec![idx]).collect();
    }

    let mut waves = Vec::new();
    let mut left = 0usize;
    let mut right = total_cards - 1;
    while left < right {
        waves.push(vec![left, right]);
        left += 1;
        right = right.saturating_sub(1);
    }
    if left == right {
        waves.push(vec![left]);
    }
    waves
}

#[derive(Clone, Copy, Default)]
struct OverlayPauseState {
    paused: bool,
    previous_lock_input: bool,
}

fn pause_game_for_overlay(state: &Rc<RefCell<AppState>>) -> OverlayPauseState {
    let mut st = state.borrow_mut();
    let in_game_view = st
        .view_stack
        .as_ref()
        .and_then(|stack| stack.visible_child_name())
        .as_deref()
        == Some("game");
    if !in_game_view || st.timer_handle.is_none() {
        return OverlayPauseState::default();
    }

    let pause_state = OverlayPauseState {
        paused: true,
        previous_lock_input: st.lock_input,
    };
    stop_timer(&mut st);
    st.lock_input = true;
    if let Some(subtitle) = &st.title_game_subtitle {
        subtitle.set_text("PAUSED");
    }
    pause_state
}

fn resume_game_after_overlay(state: &Rc<RefCell<AppState>>, pause_state: OverlayPauseState) {
    if !pause_state.paused {
        return;
    }

    let should_resume_timer = {
        let mut st = state.borrow_mut();
        let in_game_view = st
            .view_stack
            .as_ref()
            .and_then(|stack| stack.visible_child_name())
            .as_deref()
            == Some("game");
        if !in_game_view {
            return;
        }

        st.lock_input = pause_state.previous_lock_input;
        update_subtitle(&st);
        st.timer_handle.is_none() && !st.preview_active && st.active_session_started
    };

    if should_resume_timer {
        start_timer(state, false);
    }
}

fn refresh_continue_button_state(st: &AppState) {
    if let Some(button) = &st.continue_button {
        let has_saved = session_save::has_saved_run();
        button.set_visible(has_saved);
        button.set_sensitive(has_saved);
    }
}

fn clear_saved_run_and_refresh(st: &mut AppState) {
    session_save::clear_saved_run();
    refresh_continue_button_state(st);
}

fn save_current_run_and_refresh(st: &AppState) {
    session_save::save_current_run(st);
    refresh_continue_button_state(st);
}

fn mark_run_dirty(st: &mut AppState) {
    if st.active_session_started {
        save_current_run_and_refresh(st);
    }
}

fn continue_last_run(state: &Rc<RefCell<AppState>>) {
    let Some(saved_run) = session_save::load_saved_run() else {
        let st = state.borrow();
        refresh_continue_button_state(&st);
        return;
    };

    {
        let mut st = state.borrow_mut();
        stop_timer(&mut st);
        stop_preview(&mut st);
        st.tri_level = saved_run.tri_level.clamp(1, 4);
        st.recall_level = saved_run.recall_level.clamp(1, 4);
        st.set_difficulty(saved_run.difficulty);
        if saved_run.difficulty == Difficulty::RecallMode {
            st.infinite_round = saved_run.infinite_round.max(1);
        }
        if st.tiles.len() != saved_run.tiles.len() {
            clear_saved_run_and_refresh(&mut st);
            return;
        }
        st.tiles = saved_run.tiles;
        st.flipped_indices = saved_run
            .flipped_indices
            .into_iter()
            .filter(|idx| *idx < st.tiles.len() && st.tiles[*idx].status == TileStatus::Flipped)
            .collect();
        st.seconds_elapsed = saved_run.seconds_elapsed;
        st.run_mismatches = saved_run.run_mismatches;
        st.run_matches = saved_run.run_matches;
        st.impossible_mismatch_count = saved_run.impossible_mismatch_count;
        st.impossible_punish_stage = saved_run.impossible_punish_stage;
        st.impossible_last_first_index = saved_run.impossible_last_first_index;
        st.impossible_same_first_streak = saved_run.impossible_same_first_streak;
        st.preview_active = false;
        st.preview_remaining_ms = 0;
        st.lock_input = false;
        st.active_session_started = true;
    }

    rebuild_board(state);

    {
        let st = state.borrow();
        for idx in 0..st.grid_buttons.len() {
            let button = st.grid_buttons[idx].clone();
            clear_flip_classes(&button);
            button.remove_css_class("matched");
            button.remove_css_class("active");
            button.remove_css_class("mismatch-shake");
            button.remove_css_class("match-bump");
            if idx < st.tiles.len() {
                match st.tiles[idx].status {
                    TileStatus::Matched => button.add_css_class("matched"),
                    TileStatus::Flipped => button.add_css_class("active"),
                    TileStatus::Hidden => {}
                }
            }
            redraw_button_child(&button);
        }
        update_subtitle(&st);
    }

    set_header_game(state);
    {
        let st = state.borrow();
        if let Some(stack) = &st.view_stack {
            stack.set_transition_type(gtk::StackTransitionType::SlideLeft);
            stack.set_visible_child_name("game");
        }
    }
    start_timer(state, false);
}

fn handle_tile_click_result(state: &Rc<RefCell<AppState>>, game_id: u64, indices: Vec<usize>) {
    let mut st = state.borrow_mut();
    let will_finish = st
        .tiles
        .iter()
        .enumerate()
        .all(|(i, t)| t.status == TileStatus::Matched || indices.contains(&i));
    let is_infinite_mode = infinite::is_infinite(st.difficulty);

    if will_finish
        && is_infinite_mode
        && let Some((next_milestone_difficulty, next_milestone_value)) =
            infinite_flow::infinite_milestone_value(st.infinite_round.saturating_add(1))
        && let Some(subtitle) = &st.title_game_subtitle
    {
        infinite_flow::set_infinite_milestone_subtitle(
            subtitle,
            next_milestone_difficulty,
            next_milestone_value,
        );
    }

    if will_finish
        && !is_infinite_mode
        && let Some(container) = &st.board_container
    {
        container.add_css_class("victory-pending");
    }

    for &idx in &indices {
        st.tiles[idx].status = TileStatus::Matched;
        clear_flip_classes(&st.grid_buttons[idx]);
        st.grid_buttons[idx].remove_css_class("active");
        st.grid_buttons[idx].add_css_class("matched");
        redraw_button_child(&st.grid_buttons[idx]);
    }
    st.flipped_indices.clear();
    st.lock_input = false;

    if st.tiles.iter().all(|t| t.status == TileStatus::Matched) {
        if is_infinite_mode {
            register_infinite_round_result(&mut st);
            save_current_run_and_refresh(&st);
        } else {
            register_non_infinite_result(&mut st);
            st.active_session_started = false;
            clear_saved_run_and_refresh(&mut st);
        }
        let cascade_start_delay_ms = victory_cascade_start_delay_ms(&st);
        stop_timer(&mut st);
        drop(st);
        if is_infinite_mode {
            let state_next = state.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(INFINITE_PRE_TRANSITION_WAIT_MS),
                move || {
                    infinite_flow::schedule_infinite_round_transition(&state_next, game_id);
                    glib::ControlFlow::Break
                },
            );
        } else {
            let state_victory = state.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(cascade_start_delay_ms),
                move || {
                    schedule_win_cascade_and_continue(&state_victory, game_id);
                    glib::ControlFlow::Break
                },
            );
        }
    } else {
        schedule_match_bump(state, indices.clone(), game_id);
    }
}

fn schedule_mismatch_reset(
    state: &Rc<RefCell<AppState>>,
    indices: Vec<usize>,
    game_id: u64,
    mismatch_pause_ms: u64,
    penalty_plan: Option<classic_penalties::PunishmentPlan>,
) {
    let state_clone = state.clone();
    glib::timeout_add_local(
        std::time::Duration::from_millis(mismatch_pause_ms),
        move || {
            let st = state_clone.borrow();
            if st.game_id != game_id {
                return glib::ControlFlow::Break;
            }
            for &idx in &indices {
                if let Some(button) = st.grid_buttons.get(idx) {
                    button.remove_css_class("mismatch-shake");
                    clear_flip_classes(button);
                    button.add_css_class("flip-hide");
                    redraw_button_child(button);
                }
            }
            drop(st);

            let state_swap = state_clone.clone();
            let indices_swap = indices.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(FLIP_PHASE_MS),
                move || {
                    let mut st = state_swap.borrow_mut();
                    if st.game_id != game_id {
                        return glib::ControlFlow::Break;
                    }
                    for &idx in &indices_swap {
                        st.tiles[idx].status = TileStatus::Hidden;
                        st.grid_buttons[idx].remove_css_class("active");
                        play_flip_show(&mut st, idx);
                    }
                    glib::ControlFlow::Break
                },
            );

            let state_finish = state_clone.clone();
            let indices_finish = indices.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(FLIP_PHASE_MS * 2),
                move || {
                    let mut st = state_finish.borrow_mut();
                    if st.game_id != game_id {
                        return glib::ControlFlow::Break;
                    }
                    for &idx in &indices_finish {
                        clear_flip_classes(&st.grid_buttons[idx]);
                        st.grid_buttons[idx].remove_css_class("active");
                        st.grid_buttons[idx].remove_css_class("mismatch-shake");
                        redraw_button_child(&st.grid_buttons[idx]);
                    }
                    if let Some(punishment) = penalty_plan {
                        let mut rotate_indices = Vec::new();
                        let hidden_count = st
                            .tiles
                            .iter()
                            .filter(|tile| tile.status == TileStatus::Hidden)
                            .count();
                        let hard_endgame_reshuffle_fast =
                            punishment.source_difficulty == Difficulty::Hard
                                && punishment.reshuffle_hidden
                                && hidden_count.saturating_mul(3) <= st.tiles.len();
                        if punishment.reshuffle_hidden {
                            for idx in 0..st.tiles.len() {
                                if st.tiles[idx].status == TileStatus::Hidden {
                                    let button = st.grid_buttons[idx].clone();
                                    clear_flip_classes(&button);
                                    button.remove_css_class("reshuffle-flip");
                                    button.add_css_class("reshuffle-flip");
                                    if hard_endgame_reshuffle_fast {
                                        button.add_css_class("hard-reshuffle-fast");
                                    }
                                    redraw_button_child(&button);
                                    rotate_indices.push(idx);
                                }
                            }
                        }
                        st.flipped_indices.clear();
                        st.lock_input = true;
                        drop(st);

                        let state_mix_finish = state_finish.clone();
                        let rotate_indices_finish = rotate_indices.clone();
                        let punishment_reshuffle = punishment.reshuffle_hidden;
                        glib::timeout_add_local(
                            std::time::Duration::from_millis(if punishment_reshuffle {
                                if hard_endgame_reshuffle_fast {
                                    HARD_ENDGAME_RESHUFFLE_FLIP_MS
                                } else {
                                    CLASSIC_RESHUFFLE_FLIP_MS
                                }
                            } else {
                                0
                            }),
                            move || {
                                let mut st = state_mix_finish.borrow_mut();
                                if st.game_id != game_id {
                                    return glib::ControlFlow::Break;
                                }
                                for &idx in &rotate_indices_finish {
                                    if idx < st.grid_buttons.len() {
                                        let button = st.grid_buttons[idx].clone();
                                        button.remove_css_class("hard-reshuffle-fast");
                                        button.remove_css_class("reshuffle-flip");
                                        clear_flip_classes(&button);
                                        redraw_button_child(&button);
                                    }
                                }

                                if punishment_reshuffle {
                                    // Punishment: reshuffle hidden cards first.
                                    st.reshuffle_hidden_tiles();
                                }

                                // Show only a random subset after reshuffle to force real memory.
                                use rand::seq::SliceRandom;
                                let mut hidden_indices: Vec<usize> = st
                                    .tiles
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(idx, tile)| {
                                        (tile.status == TileStatus::Hidden).then_some(idx)
                                    })
                                    .collect();
                                let mut rng = rand::rng();
                                hidden_indices.shuffle(&mut rng);
                                let reveal_indices: Vec<usize> = if punishment.reveal_all_hidden {
                                    hidden_indices
                                } else {
                                    let reveal_count =
                                        punishment.reveal_count.min(hidden_indices.len());
                                    hidden_indices.into_iter().take(reveal_count).collect()
                                };

                                for &idx in &reveal_indices {
                                    st.tiles[idx].status = TileStatus::Flipped;
                                    st.grid_buttons[idx].add_css_class("active");
                                    play_flip_show(&mut st, idx);
                                }
                                st.flipped_indices.clear();
                                st.lock_input = true;
                                drop(st);

                                let state_hide_start = state_mix_finish.clone();
                                let reveal_indices_start = reveal_indices.clone();
                                glib::timeout_add_local(
                                    std::time::Duration::from_millis(punishment.reveal_ms),
                                    move || {
                                        let st = state_hide_start.borrow();
                                        if st.game_id != game_id {
                                            return glib::ControlFlow::Break;
                                        }
                                        for &idx in &reveal_indices_start {
                                            if let Some(button) = st.grid_buttons.get(idx) {
                                                clear_flip_classes(button);
                                                button.add_css_class("flip-hide");
                                                redraw_button_child(button);
                                            }
                                        }
                                        drop(st);

                                        let state_hide_mid = state_hide_start.clone();
                                        let reveal_indices_mid = reveal_indices_start.clone();
                                        glib::timeout_add_local(
                                            std::time::Duration::from_millis(FLIP_PHASE_MS),
                                            move || {
                                                let mut st = state_hide_mid.borrow_mut();
                                                if st.game_id != game_id {
                                                    return glib::ControlFlow::Break;
                                                }
                                                for &idx in &reveal_indices_mid {
                                                    if idx < st.tiles.len() {
                                                        st.tiles[idx].status = TileStatus::Hidden;
                                                    }
                                                    if idx < st.grid_buttons.len() {
                                                        st.grid_buttons[idx]
                                                            .remove_css_class("active");
                                                        play_flip_show(&mut st, idx);
                                                    }
                                                }
                                                glib::ControlFlow::Break
                                            },
                                        );

                                        let state_hide_finish = state_hide_start.clone();
                                        let reveal_indices_finish = reveal_indices_start.clone();
                                        glib::timeout_add_local(
                                            std::time::Duration::from_millis(FLIP_PHASE_MS * 2),
                                            move || {
                                                let mut st = state_hide_finish.borrow_mut();
                                                if st.game_id != game_id {
                                                    return glib::ControlFlow::Break;
                                                }
                                                for &idx in &reveal_indices_finish {
                                                    if let Some(button) = st.grid_buttons.get(idx)
                                                    {
                                                        clear_flip_classes(button);
                                                        redraw_button_child(button);
                                                    }
                                                }
                                                st.flipped_indices.clear();
                                                st.lock_input = false;
                                                mark_run_dirty(&mut st);
                                                glib::ControlFlow::Break
                                            },
                                        );

                                        glib::ControlFlow::Break
                                    },
                                );

                                glib::ControlFlow::Break
                            },
                        );
                        glib::ControlFlow::Break
                    } else {
                        st.flipped_indices.clear();
                        st.lock_input = false;
                        mark_run_dirty(&mut st);
                        glib::ControlFlow::Break
                    }
                },
            );
            glib::ControlFlow::Break
        },
    );
}

fn schedule_match_bump(state: &Rc<RefCell<AppState>>, indices: Vec<usize>, game_id: u64) {
    let state_bump_start = state.clone();
    let indices_start = indices.clone();
    glib::timeout_add_local(
        std::time::Duration::from_millis(MATCH_BUMP_DELAY_MS),
        move || {
            let st = state_bump_start.borrow();
            if st.game_id != game_id {
                return glib::ControlFlow::Break;
            }
            for &idx in &indices_start {
                if let Some(button) = st.grid_buttons.get(idx) {
                    button.remove_css_class("match-bump");
                    button.add_css_class("match-bump");
                }
            }

            let state_bump_end = state_bump_start.clone();
            let indices_end = indices_start.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(MATCH_BUMP_DURATION_MS),
                move || {
                    let st = state_bump_end.borrow();
                    if st.game_id != game_id {
                        return glib::ControlFlow::Break;
                    }
                    for &idx in &indices_end {
                        if let Some(button) = st.grid_buttons.get(idx) {
                            button.remove_css_class("match-bump");
                        }
                    }
                    glib::ControlFlow::Break
                },
            );

            glib::ControlFlow::Break
        },
    );
}

fn schedule_win_cascade_and_continue(state: &Rc<RefCell<AppState>>, game_id: u64) {
    let (total_cards, profile) = {
        let mut st = state.borrow_mut();
        st.lock_input = true;
        if let Some(container) = &st.board_container {
            container.add_css_class("no-hover");
        }
        (st.grid_buttons.len(), cascade_profile_for(&st))
    };
    let (cascade_step_ms, post_cascade_pause_ms) = balanced_cascade_timings(total_cards, profile);
    let waves = build_cascade_waves(total_cards, profile.dual_corner_wave);

    for (wave_idx, wave_indices) in waves.iter().enumerate() {
        let wave_indices_hide = wave_indices.clone();
        let state_step = state.clone();
        glib::timeout_add_local(
            std::time::Duration::from_millis(wave_idx as u64 * cascade_step_ms),
            move || {
                let st = state_step.borrow_mut();
                let is_in_game = st.view_stack.as_ref()
                    .and_then(|s| s.visible_child_name())
                    .as_deref() == Some("game");
                
                if st.game_id != game_id || !is_in_game {
                    return glib::ControlFlow::Break;
                }
                for &idx in &wave_indices_hide {
                    if idx < st.grid_buttons.len() {
                        st.grid_buttons[idx].remove_css_class("matched");
                        st.grid_buttons[idx].remove_css_class("active");
                    }
                    if let Some(button) = st.grid_buttons.get(idx) {
                        button.add_css_class("victory-cascade");
                        clear_flip_classes(button);
                        button.add_css_class("flip-hide");
                        redraw_button_child(button);
                    }
                }
                glib::ControlFlow::Break
            },
        );

        let wave_indices_show = wave_indices.clone();
        let state_step_back = state.clone();
        glib::timeout_add_local(
            std::time::Duration::from_millis(wave_idx as u64 * cascade_step_ms + FLIP_PHASE_MS),
            move || {
                let mut st = state_step_back.borrow_mut();
                let is_in_game = st.view_stack.as_ref()
                    .and_then(|s| s.visible_child_name())
                    .as_deref() == Some("game");

                if st.game_id != game_id || !is_in_game {
                    return glib::ControlFlow::Break;
                }
                for &idx in &wave_indices_show {
                    if idx < st.tiles.len() {
                        st.tiles[idx].status = TileStatus::Hidden;
                    }
                    if idx < st.grid_buttons.len() {
                        st.grid_buttons[idx].remove_css_class("matched");
                        st.grid_buttons[idx].remove_css_class("active");
                        play_flip_show(&mut st, idx);
                    }
                }
                glib::ControlFlow::Break
            },
        );
    }

    let wave_count = waves.len();
    let cascade_span_ms = wave_count.saturating_sub(1) as u64 * cascade_step_ms;
    let total_delay = cascade_span_ms + FLIP_PHASE_MS * 2 + post_cascade_pause_ms;
    let state_end = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(total_delay), move || {
        let mut st = state_end.borrow_mut();
        let is_in_game = st.view_stack.as_ref()
            .and_then(|s| s.visible_child_name())
            .as_deref() == Some("game");

        if st.game_id != game_id || !is_in_game {
            return glib::ControlFlow::Break;
        }
        if let Some(container) = &st.board_container {
            container.remove_css_class("victory-pending");
        }
        for button in &st.grid_buttons {
            clear_flip_classes(button);
            button.remove_css_class("victory-cascade");
            redraw_button_child(button);
        }
        st.lock_input = false;
        drop(st);
        show_victory(&state_end);
        glib::ControlFlow::Break
    });
}

pub fn run() {
    glib::set_prgname(Some("io.basshift.Recall"));
    let app = adw::Application::builder()
        .application_id("io.basshift.Recall")
        .build();

    app.connect_activate(move |app| {
        load_css();

        let state = Rc::new(RefCell::new(AppState::new()));

        let instructions_action = SimpleAction::new("instructions", None);
        instructions_action.connect_activate({
            let app = app.clone();
            let state = state.clone();
            move |_, _| {
                let pause_state = pause_game_for_overlay(&state);
                let dialog = show_instructions_dialog(&app);
                let state_resume = state.clone();
                dialog.connect_response(None, move |_, _| {
                    resume_game_after_overlay(&state_resume, pause_state);
                });
            }
        });
        app.add_action(&instructions_action);

        let about_action = SimpleAction::new("about", None);
        about_action.connect_activate({
            let app = app.clone();
            let state = state.clone();
            move |_, _| {
                let pause_state = pause_game_for_overlay(&state);
                let dialog = show_about_dialog(&app);
                let state_resume = state.clone();
                dialog.connect_closed(move |_| {
                    resume_game_after_overlay(&state_resume, pause_state);
                });
            }
        });
        app.add_action(&about_action);

        let score_action = SimpleAction::new("score", None);
        score_action.connect_activate({
            let app = app.clone();
            let state = state.clone();
            move |_, _| {
                let pause_state = pause_game_for_overlay(&state);
                let dialog = show_memory_dialog(&state, &app);
                let state_resume = state.clone();
                dialog.connect_closed(move |_| {
                    resume_game_after_overlay(&state_resume, pause_state);
                });
            }
        });
        app.add_action(&score_action);

        let quit_action = SimpleAction::new("quit", None);
        quit_action.connect_activate({
            let app = app.clone();
            move |_, _| app.quit()
        });
        app.add_action(&quit_action);

        let dynamic_css_provider = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &dynamic_css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let title_menu = gtk::Label::new(None);
        title_menu.set_markup("<b>Recall</b>");
        title_menu.set_halign(gtk::Align::Center);

        let title_game_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        title_game_box.set_valign(gtk::Align::Center);
        title_game_box.set_halign(gtk::Align::Center);
        title_game_box.set_hexpand(true);

        let title_game_main = gtk::Label::builder()
            .label("Recall")
            .halign(gtk::Align::Center)
            .css_classes(vec!["game-title-main"])
            .build();

        let title_game_subtitle = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Center)
            .css_classes(vec!["game-title-subtitle", "caption"])
            .build();

        title_game_box.append(&title_game_main);
        title_game_box.append(&title_game_subtitle);

            let title_victory_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            title_victory_box.set_valign(gtk::Align::Center);
            title_victory_box.set_halign(gtk::Align::Center);
        
            let title_victory_main = gtk::Label::new(Some("Recall"));
            title_victory_main.add_css_class("game-title-main");
        
            let title_victory_sub = gtk::Label::new(Some("Victory"));
            title_victory_sub.add_css_class("game-title-subtitle");
            title_victory_sub.add_css_class("caption");
        
            title_victory_box.append(&title_victory_main);
            title_victory_box.append(&title_victory_sub);
        let header = adw::HeaderBar::builder()
            .title_widget(&title_menu)
            .build();
        header.add_css_class("app-header");
        header.add_css_class("flat");

        let back_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .build();
        back_button.set_tooltip_text(Some("Back"));
        back_button.connect_clicked({
            let state = state.clone();
            move |_| {
                show_menu(&state);
            }
        });
        header.pack_start(&back_button);

        let menu_model = gio::Menu::new();
        menu_model.append(Some("Score"), Some("app.score"));
        menu_model.append(Some("Instructions"), Some("app.instructions"));
        menu_model.append(Some("About Recall"), Some("app.about"));
        menu_model.append(Some("Quit"), Some("app.quit"));
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu_model)
            .build();

        let restart_button = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .build();
        restart_button.set_tooltip_text(Some("New Game"));
        restart_button.connect_clicked({
            let state = state.clone();
            move |_| {
                restart_game(&state);
            }
        });
        let end_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        end_box.append(&restart_button);
        end_box.append(&menu_button);
        header.pack_end(&end_box);

        let view_stack = gtk::Stack::new();
        view_stack.set_hexpand(true);
        view_stack.set_vexpand(true);
        view_stack.set_hhomogeneous(false);
        view_stack.set_vhomogeneous(false);
        view_stack.set_interpolate_size(false);
        view_stack.set_transition_type(gtk::StackTransitionType::SlideLeft);
        view_stack.set_transition_duration(300);

        let game_view = build_game_view(&state);
        view_stack.add_named(&game_view, Some("game"));

        let victory_view = build_victory_view(&state);
        view_stack.add_named(&victory_view, Some("victory"));

        let menu_view = build_menu_view(&state, app);
        view_stack.add_named(&menu_view, Some("menu"));

        view_stack.set_visible_child_name("menu");
        let toolbar = adw::ToolbarView::new();
        toolbar.set_hexpand(true);
        toolbar.set_vexpand(true);
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&view_stack));

        let win = adw::ApplicationWindow::builder()
            .application(app)
            .title("Recall")
            .icon_name("io.basshift.recall")
            .default_width(860)
            .default_height(680)
            .content(&toolbar)
            .build();
        win.set_size_request(360, 560);
        win.add_css_class("app-window");

        let style_manager = adw::StyleManager::default();
        if style_manager.is_dark() {
            win.add_css_class("theme-dark");
        } else {
            win.add_css_class("theme-light");
        }
        style_manager.connect_notify_local(Some("dark"), {
            let win = win.clone();
            move |manager, _| {
                if manager.is_dark() {
                    win.remove_css_class("theme-light");
                    win.add_css_class("theme-dark");
                } else {
                    win.remove_css_class("theme-dark");
                    win.add_css_class("theme-light");
                }
            }
        });

        {
            let mut st = state.borrow_mut();
            st.view_stack = Some(view_stack.clone());
            st.header = Some(header.clone());
            st.back_button = Some(back_button);
            st.menu_button = Some(menu_button);
            st.restart_button = Some(restart_button);
            st.title_menu = Some(title_menu);
            st.title_game = Some(title_game_box.upcast::<gtk::Widget>());
            st.title_game_subtitle = Some(title_game_subtitle);
            st.title_victory = Some(title_victory_box.upcast::<gtk::Widget>());
            st.dynamic_css_provider = Some(dynamic_css_provider);
            st.records = load_records();
            refresh_continue_button_state(&st);
        }

        let global_key = gtk::EventControllerKey::new();
        global_key.set_propagation_phase(gtk::PropagationPhase::Capture);
        global_key.connect_key_pressed({
            let state = state.clone();
            move |_, key, _, mods| {
                if debug_tools::handle_debug_shortcut(&state, key, mods) {
                    return gtk::glib::Propagation::Stop;
                }
                if key == gdk::Key::Escape {
                    let st = state.borrow();
                    let in_game = st
                        .view_stack
                        .as_ref()
                        .and_then(|stack| stack.visible_child_name())
                        .as_deref()
                        == Some("game");
                    // Allow escape if input is unlocked OR if we are just in the preview phase (so user can quit early)
                    if in_game && (!st.lock_input || st.preview_active) {
                        drop(st);
                        show_menu(&state);
                        return gtk::glib::Propagation::Stop;
                    }
                }
                gtk::glib::Propagation::Proceed
            }
        });
        win.add_controller(global_key);

        win.connect_close_request({
            let state = state.clone();
            move |_| {
                let st = state.borrow();
                if st.active_session_started {
                    save_current_run_and_refresh(&st);
                }
                gtk::glib::Propagation::Proceed
            }
        });

        set_header_menu(&state);
        win.present();
    });

    app.run();
}

fn load_css() {
    static RESOURCES_INIT: Once = Once::new();
    RESOURCES_INIT.call_once(|| {
        gio::resources_register_include!("recall.gresource")
            .expect("failed to register embedded resources");
    });

    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    let icon_theme = gtk::IconTheme::for_display(&display);
    icon_theme.add_resource_path("/io/basshift/Recall/icons/hicolor");

    for resource_path in [
        "/io/basshift/Recall/style.vars.css",
        "/io/basshift/Recall/style.css",
        "/io/basshift/Recall/style.light.css",
        "/io/basshift/Recall/style.dark.css",
        "/io/basshift/Recall/style.mobile.css",
    ] {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource(resource_path);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_menu_view(state: &Rc<RefCell<AppState>>, app: &adw::Application) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.add_css_class("main-menu-root");

    let center = gtk::CenterBox::new();
    center.set_hexpand(true);
    center.set_vexpand(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    content.add_css_class("main-menu-content");

    let icon = gtk::Image::from_icon_name("io.basshift.recall");
    icon.set_pixel_size(192);
    icon.add_css_class("main-menu-icon");

    let title = gtk::Label::new(Some("Recall"));
    title.add_css_class("main-menu-title");
    title.add_css_class("title-1");

    let buttons_box = gtk::Box::new(gtk::Orientation::Vertical, 13);
    buttons_box.set_halign(gtk::Align::Center);

    let continue_button = gtk::Button::with_label("Continue Game");
    continue_button.add_css_class("main-menu-button");
    continue_button.set_size_request(164, 40);
    continue_button.set_visible(session_save::has_saved_run());
    continue_button.connect_clicked({
        let state = state.clone();
        move |_| {
            continue_last_run(&state);
        }
    });

    let new_button = gtk::Button::with_label("New Game");
    new_button.add_css_class("main-menu-button");
    new_button.set_size_request(164, 40);
    new_button.connect_clicked({
        let state = state.clone();
        let app = app.clone();
        move |_| {
            state.borrow_mut().pending_new_game_selection = true;
            show_mode_dialog(&state, &app);
        }
    });

    content.append(&icon);
    content.append(&title);
    buttons_box.append(&continue_button);
    buttons_box.append(&new_button);
    content.append(&buttons_box);

    center.set_center_widget(Some(&content));
    root.append(&center);

    state.borrow_mut().continue_button = Some(continue_button);

    root
}

fn build_game_view(state: &Rc<RefCell<AppState>>) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.add_css_class("game-root");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_halign(gtk::Align::Fill);
    content.set_valign(gtk::Align::Fill);
    content.set_margin_top(CONTENT_MARGIN);
    content.set_margin_bottom(CONTENT_MARGIN);
    content.set_margin_start(CONTENT_MARGIN);
    content.set_margin_end(CONTENT_MARGIN);

    let board_grid = build_board_for_difficulty(state);

    let board_frame = gtk::AspectFrame::new(0.5, 0.5, 1.0, false);
    board_frame.set_halign(gtk::Align::Fill);
    board_frame.set_valign(gtk::Align::Fill);
    board_frame.set_hexpand(true);
    board_frame.set_vexpand(true);

    let board_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    board_card.set_halign(gtk::Align::Fill);
    board_card.set_valign(gtk::Align::Fill);
    board_card.set_hexpand(true);
    board_card.set_vexpand(true);
    board_card.add_css_class("recall-card-container");

    board_card.connect_closure(
        "notify::width",
        false,
        glib::closure_local!(move |card: gtk::Box, _: glib::ParamSpec| {
            if card.width() < 500 {
                card.add_css_class("compact");
            } else {
                card.remove_css_class("compact");
            }
        }),
    );

    let (grid_cols, grid_rows) = {
        let st = state.borrow();
        (st.grid_cols as f32, st.grid_rows as f32)
    };
    let grid_ratio = if grid_rows > 0.0 { grid_cols / grid_rows } else { 1.0 };
    let grid_frame = gtk::AspectFrame::new(0.5, 0.5, grid_ratio, false);
    grid_frame.set_halign(gtk::Align::Fill);
    grid_frame.set_valign(gtk::Align::Fill);
    grid_frame.set_hexpand(true);
    grid_frame.set_vexpand(true);
    grid_frame.set_child(Some(&board_grid));
    board_card.append(&grid_frame);

    board_frame.set_child(Some(&board_card));
    content.append(&board_frame);
    root.append(&content);

    {
        let mut st = state.borrow_mut();
        st.board_container = Some(board_card.clone());
    }

    root
}

pub(super) fn spawn_firework_burst(layer: &gtk::Fixed, x: f64, y: f64) {
    for i in 0..8 {
        let color_idx = i % 4;
        let particle = gtk::Label::builder()
            .label("●")
            .css_classes(vec!["firework-particle", &format!("dir-{}", i), &format!("color-{}", color_idx)])
            .build();

        particle.set_can_target(false);
        layer.put(&particle, x, y);

        // Remove particle after animation ends
        glib::timeout_add_local_once(std::time::Duration::from_millis(800), {
            let layer_weak = layer.downgrade();
            let particle_weak = particle.downgrade();
            move || {
                if let (Some(layer), Some(particle)) = (layer_weak.upgrade(), particle_weak.upgrade()) {
                    layer.remove(&particle);
                }
            }
        });
    }
}

pub(super) fn stop_victory_sparks(st: &mut AppState) {
    if let Some(handle) = st.spark_timer_handle.take() {
        handle.remove();
    }
    if let Some(layer) = &st.victory_spark_layer {
        while let Some(child) = layer.first_child() {
            layer.remove(&child);
        }
    }
}

pub(super) fn start_victory_sparks(state: &Rc<RefCell<AppState>>) {
    let mut st = state.borrow_mut();
    stop_victory_sparks(&mut st);

    let layer = st.victory_spark_layer.clone();
    let state_weak = Rc::downgrade(state);
    let mut current_spot = 0;

    let handle = glib::timeout_add_local(std::time::Duration::from_millis(600), move || {
        let Some(_state) = state_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if let Some(layer) = &layer {
            // 3 specific "Great" locations: Top-Left, Top-Right, Center-Bottom
            let (x, y) = match current_spot {
                0 => (75.0, 96.0),   // Top-Left (slightly lower)
                1 => (260.0, 74.0),  // Top-Right (slightly left)
                _ => (180.0, 178.0), // Center-Bottom (slightly higher)
            };

            spawn_firework_burst(layer, x, y);
            current_spot = (current_spot + 1) % 3;
        }
        glib::ControlFlow::Continue
    });

    st.spark_timer_handle = Some(handle);
}

fn build_victory_view(state: &Rc<RefCell<AppState>>) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.add_css_class("victory-root");
    root.set_halign(gtk::Align::Fill);
    root.set_valign(gtk::Align::Fill);

    let center = gtk::CenterBox::new();
    center.set_hexpand(true);
    center.set_vexpand(true);

    let card_shell = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card_shell.set_halign(gtk::Align::Center);
    card_shell.set_valign(gtk::Align::Center);
    card_shell.add_css_class("victory-card");
    card_shell.set_size_request(280, 430);

    let card_overlay = gtk::Overlay::new();
    card_overlay.set_halign(gtk::Align::Fill);
    card_overlay.set_valign(gtk::Align::Fill);
    card_overlay.set_hexpand(true);
    card_overlay.set_vexpand(true);

    let spark_layer = gtk::Fixed::new();
    spark_layer.set_hexpand(true);
    spark_layer.set_vexpand(true);
    spark_layer.set_can_target(false);
    spark_layer.add_css_class("victory-spark-layer");
    spark_layer.set_size_request(280, 430);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    content.set_margin_top(28);
    content.set_margin_bottom(28);
    content.set_margin_start(28);
    content.set_margin_end(28);

    let rank_art = gtk::Image::from_resource("/io/basshift/Recall/victory/rank-c.svg");
    rank_art.add_css_class("victory-rank-art");
    rank_art.set_pixel_size(160);
    rank_art.set_halign(gtk::Align::Center);

    let title = gtk::Label::new(Some("Well done!"));
    title.add_css_class("victory-title");
    title.add_css_class("title-1");

    let message = gtk::Label::new(Some(""));
    message.add_css_class("victory-message");
    message.add_css_class("body");
    message.set_wrap(true);
    message.set_justify(gtk::Justification::Center);
    message.set_max_width_chars(36);

    let stats = gtk::Label::new(None);
    stats.add_css_class("victory-message");
    stats.add_css_class("body");
    stats.set_wrap(true);
    stats.set_justify(gtk::Justification::Center);
    stats.set_max_width_chars(36);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    buttons.set_halign(gtk::Align::Center);
    buttons.set_margin_top(6);

    let again_btn = gtk::Button::with_label("Play Again");
    again_btn.add_css_class("suggested-action");
    let menu_btn = gtk::Button::with_label("Main Menu");

    again_btn.connect_clicked({
        let state = state.clone();
        move |_| {
            show_game(&state);
        }
    });
    menu_btn.connect_clicked({
        let state = state.clone();
        move |_| {
            show_menu(&state);
        }
    });

    buttons.append(&again_btn);
    buttons.append(&menu_btn);

    content.append(&rank_art);
    content.append(&title);
    content.append(&message);
    content.append(&stats);
    content.append(&buttons);
    card_overlay.set_child(Some(&spark_layer));
    card_overlay.add_overlay(&content);
    card_shell.append(&card_overlay);
    center.set_center_widget(Some(&card_shell));
    root.append(&center);

    {
        let mut st = state.borrow_mut();
        st.victory_title_label = Some(title.clone());
        st.victory_message_label = Some(message.clone());
        st.victory_stats_label = Some(stats.clone());
        st.victory_rank_art = Some(rank_art.clone());
        st.victory_spark_layer = Some(spark_layer.clone());
    }

    root
}

pub fn handle_tile_click(state: &Rc<RefCell<AppState>>, index: usize) {
    let mut st = state.borrow_mut();

    if index >= st.tiles.len() {
        return;
    }

    if st.lock_input || st.tiles[index].status != TileStatus::Hidden {
        return;
    }

    // Flip the tile
    st.tiles[index].status = TileStatus::Flipped;
    play_flip_show(&mut st, index);
    st.grid_buttons[index].add_css_class("active");
    st.flipped_indices.push(index);
    if !st.active_session_started {
        st.active_session_started = true;
        save_current_run_and_refresh(&st);
    } else {
        mark_run_dirty(&mut st);
    }

    let indices = st.flipped_indices.clone();
    let game_id = st.game_id;

        match evaluate_flip_outcome(&st, &indices, index) {
            FlipOutcome::Mismatch => {
                st.run_mismatches = st.run_mismatches.saturating_add(1);
                let first_pick_index = indices.first().copied().unwrap_or(index);
                let (mismatch_pause_ms, penalty_plan) = if st.difficulty == Difficulty::Tri {
                (
                    tri_penalties::mismatch_pause_ms(st.tri_level),
                    tri_penalties::register_mismatch_and_plan_reshuffle(&mut st, first_pick_index),
                )
            } else {
                let penalty_difficulty = if infinite::is_infinite(st.difficulty) {
                    infinite::classic_difficulty_for_round(st.infinite_round)
                } else {
                    st.difficulty
                };
                (
                    classic_penalties::mismatch_pause_ms(penalty_difficulty),
                    classic_penalties::register_mismatch_and_plan_reshuffle_for(
                        &mut st,
                        first_pick_index,
                        penalty_difficulty,
                    ),
                )
            };
            st.lock_input = true;
            let state_after_flip = state.clone();
            let indices_after_flip = indices.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(FLIP_PHASE_MS), move || {
                let st = state_after_flip.borrow_mut();
                if st.game_id != game_id {
                    return glib::ControlFlow::Break;
                }
                for &idx in &indices_after_flip {
                    if let Some(button) = st.grid_buttons.get(idx) {
                        clear_flip_classes(button);
                        button.remove_css_class("mismatch-shake");
                        button.add_css_class("mismatch-shake");
                    }
                }
                drop(st);
                schedule_mismatch_reset(
                    &state_after_flip,
                    indices_after_flip.clone(),
                    game_id,
                    mismatch_pause_ms,
                    penalty_plan,
                );
                glib::ControlFlow::Break
            });
            mark_run_dirty(&mut st);
        }
        FlipOutcome::CompleteMatch => {
            st.run_matches = st.run_matches.saturating_add(1);
            if st.difficulty == Difficulty::Tri {
                tri_penalties::reset_penalty_after_match(&mut st);
            } else {
                let penalty_difficulty = if infinite::is_infinite(st.difficulty) {
                    infinite::classic_difficulty_for_round(st.infinite_round)
                } else {
                    st.difficulty
                };
                classic_penalties::reset_penalty_after_match_for(&mut st, penalty_difficulty);
            }
            st.lock_input = true;
            mark_run_dirty(&mut st);
            drop(st);
            let state_after_flip = state.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(FLIP_PHASE_MS), move || {
                let st = state_after_flip.borrow();
                if st.game_id != game_id {
                    return glib::ControlFlow::Break;
                }
                drop(st);
                handle_tile_click_result(&state_after_flip, game_id, indices.clone());
                glib::ControlFlow::Break
            });
        }
        FlipOutcome::Continue => {
            mark_run_dirty(&mut st);
        }
    }
}

fn preview_seconds_for(st: &AppState) -> f64 {
    match st.difficulty {
        Difficulty::Easy => 3.0,
        Difficulty::Medium => 2.0,
        Difficulty::Hard => 1.2,
        Difficulty::Impossible => classic_penalties::PREVIEW_SECONDS,
        Difficulty::Tri => match st.tri_level {
            1 => 3.6,
            2 => 2.6,
            3 => 1.8,
            _ => 1.4,
        },
        Difficulty::RecallMode => (2.5 - (st.infinite_round.saturating_sub(1) as f64 * 0.15)).max(0.7),
    }
}

pub(super) fn show_game_with_reveal_delay(state: &Rc<RefCell<AppState>>, reveal_delay_override_ms: Option<u64>) {
    let (needs_rebuild, preview_seconds, game_id, reveal_delay_ms, reset_timer_for_round) = {
        let mut st = state.borrow_mut();
        let was_in_game_view = st
            .view_stack
            .as_ref()
            .and_then(|stack| stack.visible_child_name())
            .as_deref()
            == Some("game");
        let is_infinite_mode = infinite::is_infinite(st.difficulty);
        let reset_timer_for_round = !is_infinite_mode || !was_in_game_view;
        st.reset_game();
        stop_timer(&mut st);
        stop_preview(&mut st);
        stop_victory_sparks(&mut st);
        if reset_timer_for_round {
            st.seconds_elapsed = 0;
        }
        st.lock_input = true;
        if let Some(layer) = &st.victory_spark_layer {
            layer.remove_css_class("active");
        }
        let reveal_delay_ms = if let Some(stack) = &st.view_stack {
            if stack.visible_child_name().as_deref() == Some("game") {
                PREVIEW_REVEAL_MIN_DELAY_MS
            } else {
                (stack.transition_duration() as u64 + 40).max(PREVIEW_REVEAL_MIN_DELAY_MS)
            }
        } else {
            PREVIEW_REVEAL_MIN_DELAY_MS
        };
        (
            st.grid_buttons.len() != st.tiles.len(),
            preview_seconds_for(&st),
            st.game_id,
            reveal_delay_override_ms.unwrap_or(reveal_delay_ms),
            reset_timer_for_round,
        )
    };

    if needs_rebuild {
        rebuild_board(state);
    }

    {
        let mut st = state.borrow_mut();
        if let Some(container) = &st.board_container {
            container.remove_css_class("no-hover");
            container.remove_css_class("victory-pending");
            container.remove_css_class("infinite-level-swap-out");
            container.remove_css_class("infinite-level-swap-in");
            if infinite::is_infinite(st.difficulty) {
                container.add_css_class("mode-infinite");
            } else {
                container.remove_css_class("mode-infinite");
            }
        }
        // Start face-down before the global reveal.
        for i in 0..st.grid_buttons.len() {
            if let Some(tile) = st.tiles.get_mut(i) {
                tile.status = TileStatus::Hidden;
            }
            let button = &st.grid_buttons[i];
            button.remove_css_class("matched");
            button.remove_css_class("active");
            button.remove_css_class("match-bump");
            button.remove_css_class("mismatch-shake");
            clear_flip_classes(button);
            if let Some(child) = button.child() {
                child.queue_draw();
            }
        }
    }

    // Reveal all cards together after a short beat.
    let state_reveal = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(reveal_delay_ms), move || {
        let mut st = state_reveal.borrow_mut();
        if st.game_id != game_id {
            return glib::ControlFlow::Break;
        }
        for i in 0..st.grid_buttons.len() {
            if let Some(tile) = st.tiles.get_mut(i) {
                tile.status = TileStatus::Flipped;
            }
            st.grid_buttons[i].add_css_class("active");
            play_flip_show(&mut st, i);
        }
        drop(st);
        start_preview_phase(&state_reveal, preview_seconds, game_id);

        // Hide all cards together when memorize countdown ends.
        let state_hide_start = state_reveal.clone();
        glib::timeout_add_local(
            std::time::Duration::from_millis((preview_seconds * 1000.0) as u64),
            move || {
                let st = state_hide_start.borrow();
                if st.game_id != game_id || !st.preview_active {
                    return glib::ControlFlow::Break;
                }
                for button in &st.grid_buttons {
                    clear_flip_classes(button);
                    button.add_css_class("flip-hide");
                    redraw_button_child(button);
                }
                drop(st);

                let state_hide_mid = state_hide_start.clone();
                glib::timeout_add_local(
                    std::time::Duration::from_millis(FLIP_PHASE_MS),
                    move || {
                        let mut st = state_hide_mid.borrow_mut();
                        if st.game_id != game_id || !st.preview_active {
                            return glib::ControlFlow::Break;
                        }
                        for i in 0..st.grid_buttons.len() {
                            if let Some(tile) = st.tiles.get_mut(i) {
                                tile.status = TileStatus::Hidden;
                            }
                            st.grid_buttons[i].remove_css_class("active");
                            play_flip_show(&mut st, i);
                        }
                        glib::ControlFlow::Break
                    },
                );

                let state_finish = state_hide_start.clone();
                glib::timeout_add_local(
                    std::time::Duration::from_millis(FLIP_PHASE_MS * 2),
                    move || {
                        let mut st = state_finish.borrow_mut();
                        if st.game_id != game_id || !st.preview_active {
                            return glib::ControlFlow::Break;
                        }
                        for button in &st.grid_buttons {
                            clear_flip_classes(button);
                            redraw_button_child(button);
                        }
                        st.lock_input = false;
                        stop_preview(&mut st);
                        update_subtitle(&st);
                        drop(st);
                        start_timer(&state_finish, reset_timer_for_round);
                        glib::ControlFlow::Break
                    },
                );

                glib::ControlFlow::Break
            },
        );
        glib::ControlFlow::Break
    });

    set_header_game(state);
    let st = state.borrow();
    if let Some(stack) = &st.view_stack {
        stack.set_transition_type(gtk::StackTransitionType::SlideLeft);
        stack.set_visible_child_name("game");
    }
}

pub(super) fn show_game(state: &Rc<RefCell<AppState>>) {
    show_game_with_reveal_delay(state, None);
}

fn restart_game(state: &Rc<RefCell<AppState>>) {
    {
        let mut st = state.borrow_mut();
        if infinite::is_infinite(st.difficulty) {
            st.reset_infinite_round();
        }
        if classic_penalties::is_expert(st.difficulty) {
            st.impossible_mismatch_count = 0;
        }
        st.active_session_started = false;
        clear_saved_run_and_refresh(&mut st);
    }
    show_game(state);
}

pub(super) fn apply_difficulty_change(state: &Rc<RefCell<AppState>>, difficulty: Difficulty) {
    let should_rebuild = {
        let mut st = state.borrow_mut();
        if st.pending_new_game_selection {
            st.pending_new_game_selection = false;
            st.active_session_started = false;
            clear_saved_run_and_refresh(&mut st);
        }
        st.active_session_started = false;
        if st.difficulty == difficulty {
            if infinite::is_infinite(difficulty) {
                infinite::prepare_start(&mut st);
            }
            if classic_penalties::is_expert(difficulty) {
                st.impossible_mismatch_count = 0;
            }
            false
        } else {
            if infinite::is_infinite(difficulty) {
                infinite::prepare_start(&mut st);
            }
            if classic_penalties::is_expert(difficulty) {
                st.impossible_mismatch_count = 0;
            }
            st.set_difficulty(difficulty);
            true
        }
    };

    if should_rebuild {
        rebuild_board(state);
    }
    show_game(state);
}

pub(super) fn apply_tri_level_change(state: &Rc<RefCell<AppState>>, level: u8) {
    let should_refresh = {
        let mut st = state.borrow_mut();
        if st.tri_level == level.clamp(1, 4) {
            false
        } else {
            st.set_tri_level(level);
            st.difficulty == Difficulty::Tri
        }
    };

    if should_refresh {
        rebuild_board(state);
        show_game(state);
    }
}
