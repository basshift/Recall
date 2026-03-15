use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;

use super::board::build_board_grid;
use super::hud::{set_header_menu, set_header_victory, stop_preview, stop_timer};
use super::session_save;
use super::state::{AppState, Rank};
use super::app::{
    refresh_board_shell_ratio,
    refresh_continue_button_state,
    start_victory_sparks,
    stop_victory_sparks,
};

fn rank_resource_path(rank: Rank) -> &'static str {
    match rank {
        Rank::S => "/io/github/basshift/Recall/victory/rank-s.svg",
        Rank::A => "/io/github/basshift/Recall/victory/rank-a.svg",
        Rank::B => "/io/github/basshift/Recall/victory/rank-b.svg",
        Rank::C => "/io/github/basshift/Recall/victory/rank-c.svg",
    }
}

pub(super) fn rebuild_board(state: &Rc<RefCell<AppState>>) {
    let (board_container, grid_cols, grid_rows) = {
        let st = state.borrow();
        (st.board_container.clone(), st.grid_cols, st.grid_rows)
    };
    let Some(board_container) = board_container else {
        return;
    };

    while let Some(child) = board_container.first_child() {
        board_container.remove(&child);
    }
    let grid = build_board_grid(state);
    let grid_ratio = if grid_rows > 0 {
        grid_cols as f32 / grid_rows as f32
    } else {
        1.0
    };
    let grid_frame = gtk::AspectFrame::new(0.5, 0.5, grid_ratio, false);
    grid_frame.set_halign(gtk::Align::Fill);
    grid_frame.set_valign(gtk::Align::Fill);
    grid_frame.set_hexpand(true);
    grid_frame.set_vexpand(true);
    grid_frame.set_child(Some(&grid));
    board_container.append(&grid_frame);
    refresh_board_shell_ratio(state);
}

pub(super) fn show_victory(state: &Rc<RefCell<AppState>>) {
    let is_s_rank = {
        let st = state.borrow();
        if let Some(label) = &st.victory_title_label {
            label.set_text(&st.victory_title_text);
        }
        if let Some(label) = &st.victory_message_label {
            label.set_text(&st.victory_message_text);
        }
        if let Some(label) = &st.victory_stats_label {
            label.set_text(&st.victory_stats_text);
        }
        let rank = st.victory_rank;
        if let Some(image) = &st.victory_rank_art {
            if let Some(custom_resource) = &st.victory_art_resource {
                image.set_resource(Some(custom_resource));
            } else {
                image.set_resource(Some(rank_resource_path(rank)));
            }
            image.set_visible(true);
        }
        rank == Rank::S
    };
    set_header_victory(state);
    if is_s_rank {
        start_victory_sparks(state);
    } else {
        let mut st = state.borrow_mut();
        stop_victory_sparks(&mut st);
    }
    let st = state.borrow();
    if let Some(stack) = &st.view_stack {
        stack.set_transition_type(gtk::StackTransitionType::SlideLeft);
        stack.set_visible_child_name("victory");
    }
}

pub(super) fn show_menu(state: &Rc<RefCell<AppState>>) {
    {
        let mut st = state.borrow_mut();
        if st.active_session_started
            && let Err(err) = session_save::save_current_run(&st)
        {
            eprintln!("warning: failed to save current run: {err}");
        }
        // Invalidate pending async callbacks from in-game transitions.
        st.invalidate_callbacks();
        st.lock_input = false;
        st.flipped_indices.clear();
        stop_timer(&mut st);
        stop_preview(&mut st);
        stop_victory_sparks(&mut st);
        refresh_continue_button_state(&st);
    }
    set_header_menu(state);
    let st = state.borrow();
    if let Some(stack) = &st.view_stack {
        stack.set_transition_type(gtk::StackTransitionType::SlideRight);
        stack.set_visible_child_name("menu");
    }
}
