use std::cell::RefCell;
use std::rc::Rc;

use gio::Menu;
use gtk4::glib;
use gtk4::prelude::*;

use crate::i18n::tr;

use super::infinite;
use super::state::{AppState, Difficulty};

fn refresh_header_action_button(st: &AppState) {
    let Some(button) = &st.restart_button else {
        return;
    };

    button.set_child(None::<&gtk4::Widget>);
    if infinite::is_infinite(st.difficulty) {
        let icon = gtk4::Image::builder()
            .icon_name("media-playback-stop-symbolic")
            .pixel_size(16)
            .build();
        button.set_child(Some(&icon));
        button.set_tooltip_text(Some(&tr("End run")));
        button.set_sensitive(st.active_session_started && !st.preview_active && !st.lock_input);
        button.set_visible(true);
    } else {
        button.set_visible(false);
    }
}

fn refresh_header_menu_button(st: &AppState, include_game_action: bool) {
    let Some(menu_button) = &st.menu_button else {
        return;
    };

    let menu_model = Menu::new();
    if include_game_action {
        menu_model.append(Some(&tr("Restart game")), Some("app.game-action"));
    }
    menu_model.append(Some(&tr("Score")), Some("app.score"));
    menu_model.append(Some(&tr("Preferences")), Some("app.preferences"));
    menu_model.append(Some(&tr("Keyboard Shortcuts")), Some("win.show-help-overlay"));
    menu_model.append(Some(&tr("How to Play")), Some("app.instructions"));
    menu_model.append(Some(&tr("About Recall")), Some("app.about"));
    menu_button.set_menu_model(Some(&menu_model));
}

pub(super) fn set_header_menu(state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    if let Some(header) = &st.header {
        let empty_title = gtk4::Label::new(None);
        empty_title.set_text("");
        header.set_title_widget(Some(&empty_title));
    }
    if let Some(back) = &st.back_button {
        back.set_visible(false);
    }
    if let Some(timer_label) = &st.header_timer_label {
        timer_label.set_visible(false);
    }
    if let Some(restart) = &st.restart_button {
        restart.set_visible(false);
    }
    refresh_header_menu_button(&st, false);
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
    refresh_header_action_button(&st);
    refresh_header_menu_button(&st, true);
}

pub(super) fn set_header_victory(state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    if let (Some(header), Some(title)) = (&st.header, &st.title_victory) {
        header.set_title_widget(Some(title));
    }
    if let Some(back) = &st.back_button {
        back.set_visible(true);
    }
    if let Some(timer_label) = &st.header_timer_label {
        timer_label.set_visible(false);
    }
    if let Some(restart) = &st.restart_button {
        restart.set_visible(false);
    }
    refresh_header_menu_button(&st, false);
}

pub(super) fn update_subtitle(st: &AppState) {
    refresh_header_action_button(st);
    let mode_label = if st.difficulty == Difficulty::Trio {
        format!("{} · {}", tr("Trio"), tr(infinite::level_name(st.trio_level)))
    } else if infinite::is_infinite(st.difficulty) {
        infinite::mode_label(st)
    } else {
        format!("{} · {}", tr("Classic"), tr(st.difficulty.name()))
    };
    let timer_text = if st.preview_active {
        let remain = st.preview_remaining_ms as f64 / 1000.0;
        format!("{:.1}s", remain)
    } else {
        let mins = st.seconds_elapsed / 60;
        let secs = st.seconds_elapsed % 60;
        format!("{:02}:{:02}", mins, secs)
    };

    if let Some(subtitle) = &st.title_game_subtitle {
        if st.compact_layout {
            subtitle.set_text(&mode_label);
        } else {
            subtitle.set_text(&format!("{} · {}", mode_label, timer_text));
        }
    }
    if let Some(timer_label) = &st.header_timer_label {
        let show_mobile_timer = st.compact_layout
            && (st.preview_active
                || (st.active_session_started
                    && (st.timer_handle.is_some() || st.seconds_elapsed > 0)));
        timer_label.set_visible(show_mobile_timer);
        timer_label.set_text(&timer_text);
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
