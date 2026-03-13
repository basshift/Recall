use std::cell::RefCell;
use std::rc::Rc;
use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use super::state::{AppState, TileStatus, Difficulty};
use super::app::{clear_flip_classes, play_flip_show, redraw_button_child, show_game_with_reveal_delay};
use super::infinite;
use crate::i18n::tr;

const FLIP_PHASE_MS: u64 = 260;
const INFINITE_ROUND_TRANSITION_MS: u64 = 620;
const INFINITE_LEVEL_SWAP_OUT_MS: u64 = 520;
const INFINITE_POST_TRANSITION_WAIT_MS: u64 = 0;
const INFINITE_MILESTONE_HOLD_MS: u64 = 0;

pub fn schedule_infinite_round_transition(state: &Rc<RefCell<AppState>>, game_id: u64) {
    {
        let mut st = state.borrow_mut();
        if st.game_id != game_id {
            return;
        }
        st.lock_input = true;
        st.flipped_indices.clear();
    }

    let state_hide_start = state.clone();
    glib::timeout_add_local(
        std::time::Duration::from_millis(0),
        move || {
            let st = state_hide_start.borrow();
            if st.game_id != game_id {
                return glib::ControlFlow::Break;
            }
            for button in &st.grid_buttons {
                clear_flip_classes(button);
                button.remove_css_class("reshuffle-flip");
                button.remove_css_class("match-bump");
                button.remove_css_class("mismatch-shake");
                button.remove_css_class("matched");
                button.remove_css_class("matched-dim");
                button.remove_css_class("active");
                button.add_css_class("flip-hide");
                redraw_button_child(button);
            }
            drop(st);

            let state_hide_mid = state_hide_start.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(FLIP_PHASE_MS),
                move || {
                    let mut st = state_hide_mid.borrow_mut();
                    if st.game_id != game_id {
                        return glib::ControlFlow::Break;
                    }
                    for i in 0..st.grid_buttons.len() {
                        if let Some(tile) = st.tiles.get_mut(i) {
                            tile.status = TileStatus::Hidden;
                        }
                        st.grid_buttons[i].remove_css_class("matched");
                        st.grid_buttons[i].remove_css_class("matched-dim");
                        st.grid_buttons[i].remove_css_class("active");
                        play_flip_show(&mut st, i);
                    }
                    glib::ControlFlow::Break
                },
            );

            let state_hide_finish = state_hide_start.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(FLIP_PHASE_MS * 2),
                move || {
                    let st = state_hide_finish.borrow();
                    if st.game_id != game_id {
                        return glib::ControlFlow::Break;
                    }
                    for button in &st.grid_buttons {
                        clear_flip_classes(button);
                        button.remove_css_class("infinite-round-flip");
                        redraw_button_child(button);
                    }
                    let next_level = infinite::projected_level_for_next_round(&st);
                    let level_up_transition = next_level != st.infinite_level;
                    if level_up_transition
                        && let Some(subtitle) = &st.title_game_subtitle
                    {
                        set_level_up_subtitle(subtitle, next_level);
                    }
                    if level_up_transition
                        && let Some(container) = &st.board_container
                    {
                        container.remove_css_class("infinite-level-swap-in");
                        container.remove_css_class("infinite-level-swap-out");
                        container.add_css_class("infinite-level-swap-out");
                    }
                    if !level_up_transition {
                        for button in &st.grid_buttons {
                            button.add_css_class("infinite-round-flip");
                            redraw_button_child(button);
                        }
                    }
                    drop(st);

                    let state_apply = state_hide_finish.clone();
                    if level_up_transition {
                        glib::timeout_add_local(
                            std::time::Duration::from_millis(INFINITE_LEVEL_SWAP_OUT_MS),
                            move || {
                                finalize_infinite_transition(&state_apply, game_id, true);
                                glib::ControlFlow::Break
                            },
                        );
                    } else {
                        glib::timeout_add_local(
                            std::time::Duration::from_millis(INFINITE_ROUND_TRANSITION_MS),
                            move || {
                                finalize_infinite_transition(&state_apply, game_id, false);
                                glib::ControlFlow::Break
                            },
                        );
                    }

                    glib::ControlFlow::Break
                },
            );

            glib::ControlFlow::Break
        },
    );
}

pub fn finalize_infinite_transition(
    state: &Rc<RefCell<AppState>>,
    game_id: u64,
    apply_level_swap_in: bool,
) {
    let mut st = state.borrow_mut();
    if st.game_id != game_id {
        return;
    }
    for button in &st.grid_buttons {
        button.remove_css_class("reshuffle-flip");
        clear_flip_classes(button);
        redraw_button_child(button);
    }

    let _ = infinite::advance_round(&mut st);
    let milestone = infinite_milestone_value(st.infinite_round);
    if let Some((milestone_difficulty, milestone_value)) = milestone
        && let Some(subtitle) = &st.title_game_subtitle
    {
        set_infinite_milestone_subtitle(subtitle, milestone_difficulty, milestone_value);
    }
    drop(st);

    let launch_next_round = move |state_ref: &Rc<RefCell<AppState>>, with_swap_in: bool| {
        {
            let st = state_ref.borrow();
            let in_game = st
                .view_stack
                .as_ref()
                .and_then(|stack| stack.visible_child_name())
                .as_deref()
                == Some("game");
            if st.game_id != game_id || !in_game {
                return;
            }
        }
        show_game_with_reveal_delay(state_ref, Some(INFINITE_POST_TRANSITION_WAIT_MS));
        if with_swap_in {
            let state_swap = state_ref.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(0), move || {
                let st = state_swap.borrow();
                let in_game = st
                    .view_stack
                    .as_ref()
                    .and_then(|stack| stack.visible_child_name())
                    .as_deref()
                    == Some("game");
                if st.game_id != game_id || !in_game {
                    return glib::ControlFlow::Break;
                }
                if let Some(container) = &st.board_container {
                    container.remove_css_class("infinite-level-swap-in");
                    container.add_css_class("infinite-level-swap-in");
                }
                glib::ControlFlow::Break
            });
        }
    };

    if milestone.is_some() {
        let state_next = state.clone();
        glib::timeout_add_local(
            std::time::Duration::from_millis(INFINITE_MILESTONE_HOLD_MS),
            move || {
                launch_next_round(&state_next, apply_level_swap_in);
                glib::ControlFlow::Break
            },
        );
    } else {
        launch_next_round(state, apply_level_swap_in);
    }
}

pub fn infinite_milestone_value(round: u32) -> Option<(Difficulty, u32)> {
    match infinite::classic_difficulty_for_round(round) {
        Difficulty::Hard => {
            let hard_survival = infinite::hard_survival_rounds(round);
            if hard_survival > 0 && hard_survival.is_multiple_of(5) {
                Some((Difficulty::Hard, hard_survival))
            } else {
                None
            }
        }
        Difficulty::Impossible => {
            let expert_survival = infinite::expert_survival_rounds(round);
            if expert_survival > 0 && expert_survival.is_multiple_of(5) {
                Some((Difficulty::Impossible, expert_survival))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn set_infinite_milestone_subtitle(subtitle: &gtk::Label, difficulty: Difficulty, value: u32) {
    let label = if difficulty == Difficulty::Impossible {
        tr("Expert")
    } else {
        tr("Hard")
    };
    let escaped_label = glib::markup_escape_text(&label);
    subtitle.set_markup(&format!("<b>{} x{}!</b>", escaped_label, value));
}

pub fn set_level_up_subtitle(subtitle: &gtk::Label, level: u8) {
    let level_name = tr(infinite::level_name(level));
    let escaped_heading = glib::markup_escape_text(&tr("Level up"));
    let escaped_level_name = glib::markup_escape_text(&level_name);
    subtitle.set_markup(&format!("<b>{}: {}!</b>", escaped_heading, escaped_level_name));
}
