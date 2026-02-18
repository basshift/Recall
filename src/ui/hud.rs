use std::cell::RefCell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;

use super::infinite;
use super::state::{AppState, Difficulty};

pub(super) fn set_header_menu(state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    if let (Some(header), Some(title)) = (&st.header, &st.title_menu) {
        header.set_title_widget(Some(title));
    }
    if let Some(back) = &st.back_button {
        back.set_visible(false);
    }
    if let Some(restart) = &st.restart_button {
        restart.set_visible(false);
    }
}

pub(super) fn set_header_game(state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    if let (Some(header), Some(title_box)) = (&st.header, &st.title_game) {
        update_subtitle(&st);
        header.set_title_widget(Some(title_box));
    }
    if let Some(back) = &st.back_button {
        back.set_visible(true);
    }
    if let Some(restart) = &st.restart_button {
        restart.set_visible(true);
    }
}

pub(super) fn set_header_victory(state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    if let (Some(header), Some(title)) = (&st.header, &st.title_victory) {
        header.set_title_widget(Some(title));
    }
    if let Some(back) = &st.back_button {
        back.set_visible(true);
    }
    if let Some(restart) = &st.restart_button {
        restart.set_visible(false);
    }
}

pub(super) fn update_subtitle(st: &AppState) {
    if let Some(subtitle) = &st.title_game_subtitle {
        let mode_label = if st.difficulty == Difficulty::Tri {
            format!("Tri {}", infinite::level_name(st.tri_level))
        } else if infinite::is_infinite(st.difficulty) {
            infinite::mode_label(st)
        } else {
            format!("Classic {}", st.difficulty.name())
        };
        if st.preview_active {
            let remain = st.preview_remaining_ms as f64 / 1000.0;
            subtitle.set_text(&format!("{} | Memorize {:.1}s", mode_label, remain));
        } else {
            let mins = st.seconds_elapsed / 60;
            let secs = st.seconds_elapsed % 60;
            subtitle.set_text(&format!("{} | {:02}:{:02}", mode_label, mins, secs));
        }
    }
}

pub(super) fn stop_timer(st: &mut AppState) {
    if let Some(handle) = st.timer_handle.take() {
        handle.remove();
    }
}

pub(super) fn stop_preview(st: &mut AppState) {
    st.preview_active = false;
    st.preview_remaining_ms = 0;
    if let Some(handle) = st.preview_handle.take() {
        handle.remove();
    }
}

pub(super) fn start_timer(state: &Rc<RefCell<AppState>>, reset_elapsed: bool) {
    let mut st = state.borrow_mut();
    stop_timer(&mut st);
    if reset_elapsed {
        st.seconds_elapsed = 0;
    }

    let state_clone = state.clone();
    let handle = glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
        let mut st = state_clone.borrow_mut();
        st.seconds_elapsed += 1;
        update_subtitle(&st);
        glib::ControlFlow::Continue
    });
    st.timer_handle = Some(handle);
}

pub(super) fn start_preview_phase(state: &Rc<RefCell<AppState>>, preview_seconds: f64, game_id: u64) {
    {
        let mut st = state.borrow_mut();
        stop_preview(&mut st);
        st.preview_active = true;
        st.preview_remaining_ms = (preview_seconds.max(0.1) * 1000.0) as u32;
        update_subtitle(&st);
    }

    let state_tick = state.clone();
    let tick = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        let mut st = state_tick.borrow_mut();
        if st.game_id != game_id || !st.preview_active {
            return glib::ControlFlow::Break;
        }
        st.preview_remaining_ms = st.preview_remaining_ms.saturating_sub(100);
        update_subtitle(&st);
        glib::ControlFlow::Continue
    });
    state.borrow_mut().preview_handle = Some(tick);
}
