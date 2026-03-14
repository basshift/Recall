use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;

use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use gio::SimpleAction;

use crate::i18n::tr;

use super::board::CONTENT_MARGIN;
use super::dialogs::{create_keyboard_shortcuts_overlay, show_about_dialog, show_instructions_dialog};
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
    register_infinite_run_result,
    register_non_infinite_result,
    reset_local_records,
    show_memory_dialog,
};
use super::scene::{build_board_for_difficulty, rebuild_board, show_menu, show_victory};
use super::session_save;
use super::state::{AppState, Difficulty, Rank, TileStatus};
use super::trio_penalties;
use super::debug_tools;
use super::infinite_flow;

fn show_preferences_dialog(state: &Rc<RefCell<AppState>>, app: &adw::Application) -> adw::PreferencesDialog {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title(&tr("Preferences"));
    dialog.set_can_close(true);
    dialog.set_follows_content_size(false);
    dialog.set_content_width(420);
    dialog.set_content_height(360);

    let page = adw::PreferencesPage::new();
    page.set_title(&tr("General"));

    let appearance_group = adw::PreferencesGroup::new();
    appearance_group.set_title(&tr("Appearance"));

    let theme_row = adw::ComboRow::builder()
        .title(tr("Theme"))
        .subtitle(tr("Select app color scheme"))
        .build();
    let theme_values = [tr("System"), tr("Light"), tr("Dark")];
    let theme_refs: Vec<&str> = theme_values.iter().map(|s| s.as_str()).collect();
    let theme_model = gtk::StringList::new(&theme_refs);
    theme_row.set_model(Some(&theme_model));
    let style_manager = adw::StyleManager::default();
    let initial_theme_index = match style_manager.color_scheme() {
        adw::ColorScheme::ForceLight | adw::ColorScheme::PreferLight => 1,
        adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => 2,
        _ => 0,
    };
    theme_row.set_selected(initial_theme_index);
    theme_row.connect_selected_notify(move |row| {
        let scheme = match row.selected() {
            1 => adw::ColorScheme::ForceLight,
            2 => adw::ColorScheme::ForceDark,
            _ => adw::ColorScheme::Default,
        };
        adw::StyleManager::default().set_color_scheme(scheme);
    });
    appearance_group.add(&theme_row);

    let motion_row = adw::SwitchRow::builder()
        .title(tr("Reduce motion"))
        .subtitle(tr("Turn off interface animations"))
        .build();
    if let Some(settings) = gtk::Settings::default() {
        motion_row.set_active(!settings.is_gtk_enable_animations());
    }
    motion_row.connect_active_notify(|row| {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(!row.is_active());
        }
    });
    appearance_group.add(&motion_row);

    page.add(&appearance_group);

    let data_group = adw::PreferencesGroup::new();
    data_group.set_title(&tr("Data"));
    let reset_row = adw::ActionRow::builder()
        .title(tr("Reset local records"))
        .subtitle(tr("Clear all saved scores on this device"))
        .build();
    reset_row.set_activatable(false);
    let reset_button = gtk::Button::with_label(&tr("Reset"));
    reset_button.add_css_class("destructive-action");
    reset_button.set_halign(gtk::Align::End);
    reset_button.set_valign(gtk::Align::Center);
    reset_button.set_hexpand(false);
    reset_button.set_vexpand(false);
    reset_row.add_suffix(&reset_button);
    {
        let dialog = dialog.clone();
        let state = state.clone();
        reset_button.connect_clicked(move |_| {
            let confirm = adw::AlertDialog::builder()
                .heading(tr("Reset local records"))
                .body(tr("This will permanently remove all saved scores on this device"))
                .build();
            confirm.add_response("cancel", &tr("Cancel"));
            confirm.add_response("reset", &tr("Reset"));
            confirm.set_close_response("cancel");
            confirm.set_default_response(Some("cancel"));
            confirm.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
            let dialog_after = dialog.clone();
            let state_after = state.clone();
            confirm.connect_response(None, move |_, response| {
                if response == "reset" {
                    reset_local_records(&state_after);
                    let done = adw::AlertDialog::builder()
                        .heading(tr("Records reset"))
                        .body(tr("Local scores were cleared successfully"))
                        .build();
                    done.add_response("ok", &tr("OK"));
                    done.present(Some(&dialog_after));
                }
            });
            confirm.present(Some(&dialog));
        });
    }
    data_group.add(&reset_row);
    page.add(&data_group);

    dialog.add(&page);
    dialog.present(app.active_window().as_ref());
    dialog
}

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
const MATCH_BUMP_DELAY_MS: u64 = 120;
const MATCH_BUMP_DURATION_MS: u64 = 700;
const FINAL_MATCH_DIM_SETTLE_MS: u64 = 110;
const PREVIEW_REVEAL_MIN_DELAY_MS: u64 = 500;
const VICTORY_FLIP_SHOW_DURATION_MS: u64 = 380;
const VICTORY_CASCADE_END_BUFFER_MS: u64 = 32;

fn sync_window_maximized_class(win: &adw::ApplicationWindow) {
    if win.is_maximized() {
        win.add_css_class("window-maximized");
    } else {
        win.remove_css_class("window-maximized");
    }
}

pub(super) fn refresh_board_shell_ratio(state: &Rc<RefCell<AppState>>) {
    let (board_shell, grid_cols, grid_rows, compact_layout) = {
        let st = state.borrow();
        (
            st.board_shell.clone(),
            st.grid_cols,
            st.grid_rows,
            st.compact_layout,
        )
    };
    let Some(board_shell) = board_shell else {
        return;
    };
    let ratio = if compact_layout && grid_rows > 0 {
        grid_cols as f32 / grid_rows as f32
    } else {
        1.0
    };
    board_shell.set_ratio(ratio.max(0.2));
}

fn sync_window_layout_classes(win: &adw::ApplicationWindow, state: &Rc<RefCell<AppState>>) {
    let width = win.allocated_width().max(1);
    let height = win.allocated_height().max(1);
    let compact_layout = (width < 760 && height < 620) || width < 520;
    let ultra_compact_layout = (width < 620 && height < 520) || width < 440;

    if compact_layout {
        win.add_css_class("window-compact");
    } else {
        win.remove_css_class("window-compact");
    }
    if ultra_compact_layout {
        win.add_css_class("window-ultra-compact");
    } else {
        win.remove_css_class("window-ultra-compact");
    }

    let layout_changed = {
        let mut st = state.borrow_mut();
        let changed = st.compact_layout != compact_layout;
        st.compact_layout = compact_layout;
        changed
    };
    refresh_board_shell_ratio(state);
    if layout_changed {
        let st = state.borrow();
        update_subtitle(&st);
    }
}

fn is_game_view_active(st: &AppState) -> bool {
    st.view_stack
        .as_ref()
        .and_then(|stack| stack.visible_child_name())
        .as_deref()
        == Some("game")
}

fn can_show_keyboard_focus(st: &AppState) -> bool {
    is_game_view_active(st)
        && !st.preview_active
        && !st.lock_input
        && st.tiles.iter().any(|tile| tile.status == TileStatus::Hidden)
}

fn clear_keyboard_focus(state: &Rc<RefCell<AppState>>) {
    let buttons = {
        let st = state.borrow();
        st.grid_buttons.clone()
    };
    for button in buttons {
        button.remove_css_class("kbd-focus");
    }
}

fn focused_tile_index(st: &AppState) -> Option<usize> {
    st.grid_buttons.iter().position(|button| {
        button.has_focus() || button.has_visible_focus() || button.has_css_class("kbd-focus")
    })
}

fn normalize_target_col(row: i32, col: i32, cols: i32, len: usize) -> i32 {
    let mut target_col = col.clamp(0, cols.saturating_sub(1));
    while target_col >= 0 {
        let candidate = (row * cols + target_col) as usize;
        if candidate < len {
            return target_col;
        }
        target_col -= 1;
    }
    0
}

fn focus_tile_at_index(state: &Rc<RefCell<AppState>>, index: usize) -> bool {
    let (buttons, button) = {
        let st = state.borrow();
        if !can_show_keyboard_focus(&st) {
            return false;
        }
        (st.grid_buttons.clone(), st.grid_buttons.get(index).cloned())
    };
    let Some(button) = button else {
        return false;
    };
    for (button_index, candidate) in buttons.iter().enumerate() {
        if button_index == index {
            candidate.add_css_class("kbd-focus");
        } else {
            candidate.remove_css_class("kbd-focus");
        }
    }
    button.grab_focus();
    true
}

fn move_board_focus(state: &Rc<RefCell<AppState>>, col_delta: i32, row_delta: i32) -> bool {
    let next_index = {
        let st = state.borrow();
        if !is_game_view_active(&st) || st.grid_buttons.is_empty() || st.grid_cols <= 0 {
            return false;
        }

        let current_index = focused_tile_index(&st).unwrap_or(0);
        let cols = st.grid_cols;
        let len = st.grid_buttons.len();
        let max_row = ((len as i32 - 1) / cols).max(0);

        let current_row = (current_index as i32 / cols).clamp(0, max_row);
        let current_col = (current_index as i32 % cols).clamp(0, cols.saturating_sub(1));
        let target_row = (current_row + row_delta).clamp(0, max_row);
        let desired_col = current_col + col_delta;
        let target_col = normalize_target_col(target_row, desired_col, cols, len);
        (target_row * cols + target_col) as usize
    };

    focus_tile_at_index(state, next_index)
}

fn suppress_board_hover_for_keyboard(state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    if !is_game_view_active(&st) || st.lock_input {
        return;
    }
    if let Some(container) = &st.board_container {
        container.add_css_class("no-hover");
    }
}

fn activate_focused_tile(state: &Rc<RefCell<AppState>>) -> bool {
    let tile_index = {
        let st = state.borrow();
        if !is_game_view_active(&st) || st.grid_buttons.is_empty() {
            return false;
        }
        focused_tile_index(&st).unwrap_or(0)
    };
    handle_tile_click(state, tile_index);
    true
}

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
        Difficulty::Trio => match st.trio_level.clamp(1, 4) {
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
                start_delay_ms: 500,
                base_step_ms: 112,
                base_pause_ms: 72,
                step_min_ms: 62,
                step_max_ms: 184,
                pause_min_ms: 58,
                pause_max_ms: 148,
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
    paused_during_preview: bool,
}

fn pause_game_for_overlay(state: &Rc<RefCell<AppState>>) -> OverlayPauseState {
    let mut st = state.borrow_mut();
    let in_game_view = st
        .view_stack
        .as_ref()
        .and_then(|stack| stack.visible_child_name())
        .as_deref()
        == Some("game");
    if !in_game_view {
        return OverlayPauseState::default();
    }

    let has_active_game_flow = st.timer_handle.is_some() || st.preview_active || st.lock_input;
    if !has_active_game_flow {
        return OverlayPauseState::default();
    }

    let pause_state = OverlayPauseState {
        paused: true,
        previous_lock_input: st.lock_input,
        paused_during_preview: st.preview_active,
    };
    st.lock_input = true;
    pause_state
}

fn resume_game_after_overlay(state: &Rc<RefCell<AppState>>, pause_state: OverlayPauseState) {
    if !pause_state.paused {
        return;
    }

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

    let preview_finished_while_paused = pause_state.paused_during_preview && !st.preview_active;
    st.lock_input = if preview_finished_while_paused {
        false
    } else {
        pause_state.previous_lock_input
    };
    update_subtitle(&st);
}

fn saved_run_level_name(level: u8) -> &'static str {
    match level.clamp(1, 4) {
        1 => "Easy",
        2 => "Medium",
        3 => "Hard",
        _ => "Expert",
    }
}

fn saved_run_subtitle(saved_run: &session_save::SavedRun) -> String {
    let mode_label = match saved_run.difficulty {
        Difficulty::Infinite => format!("{} {}", tr("Infinite Round"), saved_run.infinite_round.max(1)),
        Difficulty::Trio => format!("{} {}", tr("Trio"), tr(saved_run_level_name(saved_run.trio_level))),
        _ => format!("{} {}", tr("Classic"), tr(saved_run.difficulty.name())),
    };
    let mins = saved_run.seconds_elapsed / 60;
    let secs = saved_run.seconds_elapsed % 60;
    format!("{mode_label} · {mins:02}:{secs:02}")
}

fn set_continue_button_content(
    button: &gtk::Button,
    saved_run: Option<&session_save::SavedRun>,
) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.add_css_class("continue-button-content");
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);

    let title = gtk::Label::new(Some(&tr("Continue")));
    title.add_css_class("continue-button-title");
    title.set_halign(gtk::Align::Center);
    title.set_xalign(0.5);
    content.append(&title);

    if let Some(saved_run) = saved_run {
        let subtitle = gtk::Label::new(Some(&saved_run_subtitle(saved_run)));
        subtitle.add_css_class("continue-button-subtitle");
        subtitle.add_css_class("caption");
        subtitle.set_halign(gtk::Align::Center);
        subtitle.set_xalign(0.5);
        content.append(&subtitle);
    }

    button.set_child(Some(&content));
}

pub(super) fn refresh_continue_button_state(st: &AppState) {
    if let Some(button) = &st.continue_button {
        let saved_run = session_save::load_saved_run();
        let has_saved = saved_run.is_some();
        button.set_visible(has_saved);
        button.set_sensitive(has_saved);
        set_continue_button_content(button, saved_run.as_ref());
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

fn should_finalize_infinite_run(st: &AppState) -> bool {
    st.difficulty == Difficulty::Infinite
        && st.active_session_started
        && (st.seconds_elapsed > 0 || st.run_matches > 0 || st.run_mismatches > 0)
}

fn finalize_infinite_run_if_needed(st: &mut AppState) {
    if should_finalize_infinite_run(st) {
        register_infinite_run_result(st);
        st.active_session_started = false;
        clear_saved_run_and_refresh(st);
    }
}

fn prepare_infinite_finish_victory(st: &mut AppState) {
    let mins = st.seconds_elapsed / 60;
    let secs = st.seconds_elapsed % 60;
    let elapsed = format!("{mins:02}:{secs:02}");
    st.victory_art_resource = Some("/io/github/basshift/Recall/victory/finish-flag.svg".to_string());
    st.victory_title_text = tr("You chose the finish");
    st.victory_message_text = tr("Infinite on your terms");
    st.victory_stats_text = format!(
        "{}: {}\n{}: {}\n{}: {}",
        tr("Round"),
        st.infinite_round,
        tr("Milestone"),
        infinite::mode_label(st),
        tr("Time"),
        elapsed
    );
    st.victory_rank = Rank::C;
}

fn finish_infinite_run(state: &Rc<RefCell<AppState>>) {
    {
        let mut st = state.borrow_mut();
        if !should_finalize_infinite_run(&st) {
            return;
        }
        stop_timer(&mut st);
        stop_preview(&mut st);
        st.game_id = st.game_id.wrapping_add(1);
        st.lock_input = false;
        st.flipped_indices.clear();
        register_infinite_run_result(&mut st);
        prepare_infinite_finish_victory(&mut st);
        st.active_session_started = false;
        clear_saved_run_and_refresh(&mut st);
    }
    show_victory(state);
}

fn maybe_finish_infinite_run(state: &Rc<RefCell<AppState>>, app: &adw::Application) {
    let can_finish = {
        let st = state.borrow();
        should_finalize_infinite_run(&st) && !st.preview_active && !st.lock_input
    };
    if !can_finish {
        return;
    }

    let pause_state = pause_game_for_overlay(state);
    let dialog = adw::AlertDialog::builder()
        .heading(tr("End run?"))
        .body(tr("Your current Infinite score will be saved and this run will end"))
        .build();
    dialog.add_response("cancel", &tr("Cancel"));
    dialog.add_response("finish", &tr("End run"));
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("finish", adw::ResponseAppearance::Destructive);

    let state_response = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "finish" {
            finish_infinite_run(&state_response);
        } else {
            resume_game_after_overlay(&state_response, pause_state);
        }
    });

    dialog.present(app.active_window().as_ref());
}

fn should_confirm_restart(st: &AppState) -> bool {
    st.active_session_started || st.seconds_elapsed > 0 || st.run_matches > 0 || st.run_mismatches > 0
}

fn maybe_restart_game(state: &Rc<RefCell<AppState>>, app: &adw::Application) {
    let should_confirm = {
        let st = state.borrow();
        should_confirm_restart(&st)
    };

    if !should_confirm {
        restart_game(state);
        return;
    }

    let pause_state = pause_game_for_overlay(state);
    let dialog = adw::AlertDialog::builder()
        .heading(tr("Restart game?"))
        .body(tr("Your current progress will be lost and a new game will start."))
        .build();
    dialog.add_response("cancel", &tr("Cancel"));
    dialog.add_response("restart", &tr("Restart"));
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("restart", adw::ResponseAppearance::Destructive);

    let state_response = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "restart" {
            restart_game(&state_response);
        } else {
            resume_game_after_overlay(&state_response, pause_state);
        }
    });

    dialog.present(app.active_window().as_ref());
}

fn trigger_contextual_game_action(state: &Rc<RefCell<AppState>>, app: &adw::Application) {
    let in_game_view = {
        let st = state.borrow();
        is_game_view_active(&st)
    };
    if !in_game_view {
        return;
    }

    let is_infinite_mode = {
        let st = state.borrow();
        infinite::is_infinite(st.difficulty)
    };
    if is_infinite_mode {
        maybe_finish_infinite_run(state, app);
    } else {
        maybe_restart_game(state, app);
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
        st.trio_level = saved_run.trio_level.clamp(1, 4);
        st.infinite_level = saved_run.infinite_level.clamp(1, 4);
        st.set_difficulty(saved_run.difficulty);
        if saved_run.difficulty == Difficulty::Infinite {
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
            button.remove_css_class("matched-dim");
            button.remove_css_class("active");
            button.remove_css_class("mismatch-shake");
            button.remove_css_class("match-bump");
            if idx < st.tiles.len() {
                match st.tiles[idx].status {
                    TileStatus::Matched => {
                        button.add_css_class("matched");
                        button.add_css_class("matched-dim");
                    }
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
        st.grid_buttons[idx].remove_css_class("matched-dim");
        st.grid_buttons[idx].add_css_class("matched");
        redraw_button_child(&st.grid_buttons[idx]);
    }
    st.flipped_indices.clear();
    st.lock_input = false;

    if st.tiles.iter().all(|t| t.status == TileStatus::Matched) {
        drop(st);
        clear_keyboard_focus(state);
        let mut st = state.borrow_mut();
        if is_infinite_mode {
            save_current_run_and_refresh(&st);
        } else {
            register_non_infinite_result(&mut st);
            st.active_session_started = false;
            clear_saved_run_and_refresh(&mut st);
        }
        let cascade_start_delay_ms = victory_cascade_start_delay_ms(&st);
        stop_timer(&mut st);
        drop(st);
        schedule_match_bump(state, indices.clone(), game_id, true);
        let final_match_delay_ms =
            MATCH_BUMP_DELAY_MS + MATCH_BUMP_DURATION_MS + FINAL_MATCH_DIM_SETTLE_MS;
        if is_infinite_mode {
            let state_next = state.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(final_match_delay_ms + INFINITE_PRE_TRANSITION_WAIT_MS),
                move || {
                    infinite_flow::schedule_infinite_round_transition(&state_next, game_id);
                    glib::ControlFlow::Break
                },
            );
        } else {
            let state_victory = state.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(final_match_delay_ms + cascade_start_delay_ms),
                move || {
                    schedule_win_cascade_and_continue(&state_victory, game_id);
                    glib::ControlFlow::Break
                },
            );
        }
    } else {
        schedule_match_bump(state, indices.clone(), game_id, false);
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

fn schedule_match_bump(
    state: &Rc<RefCell<AppState>>,
    indices: Vec<usize>,
    game_id: u64,
    allow_dim_on_complete: bool,
) {
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
                    button.remove_css_class("matched-dim");
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
                    let victory_started = st.tiles.iter().all(|tile| tile.status == TileStatus::Matched);
                    for &idx in &indices_end {
                        if let Some(button) = st.grid_buttons.get(idx) {
                            button.remove_css_class("match-bump");
                            if !victory_started || allow_dim_on_complete {
                                button.add_css_class("matched-dim");
                            }
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
        for button in &st.grid_buttons {
            button.remove_css_class("matched-dim");
            button.remove_css_class("match-bump");
        }
        (st.grid_buttons.len(), cascade_profile_for(&st))
    };
    clear_keyboard_focus(state);
    let color_restore_ms = 220;
    let pre_cascade_bump_ms = color_restore_ms + MATCH_BUMP_DURATION_MS;

    let state_bump_start = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(color_restore_ms), move || {
        let st = state_bump_start.borrow();
        let is_in_game = st.view_stack.as_ref()
            .and_then(|s| s.visible_child_name())
            .as_deref() == Some("game");

        if st.game_id != game_id || !is_in_game {
            return glib::ControlFlow::Break;
        }
        for button in &st.grid_buttons {
            button.remove_css_class("match-bump");
            button.add_css_class("match-bump");
        }
        glib::ControlFlow::Break
    });

    let state_bump_end = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(pre_cascade_bump_ms), move || {
        let st = state_bump_end.borrow();
        let is_in_game = st.view_stack.as_ref()
            .and_then(|s| s.visible_child_name())
            .as_deref() == Some("game");

        if st.game_id != game_id || !is_in_game {
            return glib::ControlFlow::Break;
        }
        for button in &st.grid_buttons {
            button.remove_css_class("match-bump");
        }
        glib::ControlFlow::Break
    });

    let (cascade_step_ms, post_cascade_pause_ms) = balanced_cascade_timings(total_cards, profile);
    let waves = build_cascade_waves(total_cards, profile.dual_corner_wave);

    for (wave_idx, wave_indices) in waves.iter().enumerate() {
        let wave_indices_hide = wave_indices.clone();
        let state_step = state.clone();
        glib::timeout_add_local(
            std::time::Duration::from_millis(pre_cascade_bump_ms + wave_idx as u64 * cascade_step_ms),
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
                        st.grid_buttons[idx].remove_css_class("matched-dim");
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
            std::time::Duration::from_millis(
                pre_cascade_bump_ms + wave_idx as u64 * cascade_step_ms + FLIP_PHASE_MS
            ),
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
                        st.grid_buttons[idx].remove_css_class("matched-dim");
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
    let total_delay = pre_cascade_bump_ms
        + cascade_span_ms
        + FLIP_PHASE_MS
        + VICTORY_FLIP_SHOW_DURATION_MS
        + post_cascade_pause_ms
        + VICTORY_CASCADE_END_BUFFER_MS;
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
    glib::set_prgname(Some("io.github.basshift.Recall"));
    let app = adw::Application::builder()
        .application_id("io.github.basshift.Recall")
        .build();
    app.set_accels_for_action("win.show-help-overlay", &["<Primary>slash"]);
    app.set_accels_for_action("app.instructions", &["F1"]);
    app.set_accels_for_action("app.back-menu", &["<Primary>m"]);
    app.set_accels_for_action("app.game-action", &["<Primary>r"]);
    app.set_accels_for_action("app.preferences", &["<Primary>comma"]);
    app.set_accels_for_action("app.quit", &["<Primary>q"]);

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
                dialog.connect_closed(move |_| {
                    resume_game_after_overlay(&state_resume, pause_state);
                });
            }
        });
        app.add_action(&instructions_action);

        let back_menu_action = SimpleAction::new("back-menu", None);
        back_menu_action.connect_activate({
            let state = state.clone();
            move |_, _| {
                show_menu(&state);
            }
        });
        app.add_action(&back_menu_action);

        let game_action = SimpleAction::new("game-action", None);
        game_action.connect_activate({
            let app = app.clone();
            let state = state.clone();
            move |_, _| {
                maybe_restart_game(&state, &app);
            }
        });
        app.add_action(&game_action);

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

        let preferences_action = SimpleAction::new("preferences", None);
        preferences_action.connect_activate({
            let app = app.clone();
            let state = state.clone();
            move |_, _| {
                let pause_state = pause_game_for_overlay(&state);
                let dialog = show_preferences_dialog(&state, &app);
                let state_resume = state.clone();
                dialog.connect_closed(move |_| {
                    resume_game_after_overlay(&state_resume, pause_state);
                });
            }
        });
        app.add_action(&preferences_action);

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
        
            let title_victory_sub = gtk::Label::new(Some(&tr("Victory")));
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
            .icon_name("go-home-symbolic")
            .build();
        back_button.set_tooltip_text(Some(&tr("Home")));
        back_button.connect_clicked({
            let state = state.clone();
            move |_| {
                show_menu(&state);
            }
        });
        header.pack_start(&back_button);

        let header_timer_label = gtk::Label::builder()
            .label("00:00")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Center)
            .css_classes(vec!["game-header-timer", "dim-label"])
            .build();
        header_timer_label.set_visible(false);
        header.pack_start(&header_timer_label);

        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();
        let restart_button = gtk::Button::builder().has_frame(false).build();
        restart_button.add_css_class("flat");
        restart_button.connect_clicked({
            let app = app.clone();
            let state = state.clone();
            move |_| {
                trigger_contextual_game_action(&state, &app);
            }
        });
        header.pack_end(&menu_button);
        header.pack_end(&restart_button);

        let view_stack = gtk::Stack::new();
        view_stack.set_hexpand(true);
        view_stack.set_vexpand(true);
        view_stack.set_hhomogeneous(false);
        view_stack.set_vhomogeneous(false);
        view_stack.set_interpolate_size(false);
        view_stack.set_transition_type(gtk::StackTransitionType::SlideLeft);
        view_stack.set_transition_duration(300);

        {
            let mut st = state.borrow_mut();
            st.dynamic_css_provider = Some(dynamic_css_provider.clone());
        }

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
            .icon_name("io.github.basshift.Recall")
            .default_width(860)
            .default_height(680)
            .content(&toolbar)
            .build();
        let shortcuts_overlay = create_keyboard_shortcuts_overlay();
        shortcuts_overlay.set_transient_for(Some(&win));
        let overlay_pause_state = Rc::new(RefCell::new(OverlayPauseState::default()));
        shortcuts_overlay.connect_show({
            let state = state.clone();
            let overlay_pause_state = overlay_pause_state.clone();
            move |_| {
                *overlay_pause_state.borrow_mut() = pause_game_for_overlay(&state);
            }
        });
        shortcuts_overlay.connect_hide({
            let state = state.clone();
            let overlay_pause_state = overlay_pause_state.clone();
            move |_| {
                let pause_state = *overlay_pause_state.borrow();
                resume_game_after_overlay(&state, pause_state);
                *overlay_pause_state.borrow_mut() = OverlayPauseState::default();
            }
        });
        win.set_help_overlay(Some(&shortcuts_overlay));
        win.set_size_request(360, 560);
        win.add_css_class("app-window");
        sync_window_maximized_class(&win);
        win.connect_notify_local(Some("maximized"), {
            let win = win.clone();
            move |_, _| sync_window_maximized_class(&win)
        });

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
            st.header_timer_label = Some(header_timer_label);
            st.title_victory = Some(title_victory_box.upcast::<gtk::Widget>());
            st.dynamic_css_provider = Some(dynamic_css_provider);
            st.records = load_records();
            refresh_continue_button_state(&st);
        }

        let last_window_size = Rc::new(Cell::new((0, 0)));
        let state_layout = state.clone();
        let last_window_size_tick = last_window_size.clone();
        win.add_tick_callback(move |window, _| {
            let size = (window.allocated_width(), window.allocated_height());
            if size.0 > 0 && size.1 > 0 && size != last_window_size_tick.get() {
                last_window_size_tick.set(size);
                sync_window_layout_classes(window, &state_layout);
            }
            glib::ControlFlow::Continue
        });

        let global_key = gtk::EventControllerKey::new();
        global_key.set_propagation_phase(gtk::PropagationPhase::Capture);
        global_key.connect_key_pressed({
            let state = state.clone();
            move |_, key, _, mods| {
                if debug_tools::handle_debug_shortcut(&state, key, mods) {
                    return gtk::glib::Propagation::Stop;
                }
                let has_primary_modifier = mods.intersects(
                    gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::ALT_MASK
                        | gdk::ModifierType::SUPER_MASK,
                );
                if !has_primary_modifier {
                    let handled = match key {
                        gdk::Key::Up | gdk::Key::KP_Up => {
                            suppress_board_hover_for_keyboard(&state);
                            move_board_focus(&state, 0, -1)
                        }
                        gdk::Key::Down | gdk::Key::KP_Down => {
                            suppress_board_hover_for_keyboard(&state);
                            move_board_focus(&state, 0, 1)
                        }
                        gdk::Key::Left | gdk::Key::KP_Left => {
                            suppress_board_hover_for_keyboard(&state);
                            move_board_focus(&state, -1, 0)
                        }
                        gdk::Key::Right | gdk::Key::KP_Right => {
                            suppress_board_hover_for_keyboard(&state);
                            move_board_focus(&state, 1, 0)
                        }
                        gdk::Key::space | gdk::Key::Return | gdk::Key::KP_Enter => {
                            activate_focused_tile(&state)
                        }
                        _ => false,
                    };
                    if handled {
                        return gtk::glib::Propagation::Stop;
                    }
                }
                if key == gdk::Key::Escape {
                    let st = state.borrow();
                    let in_game = is_game_view_active(&st);
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
    static CSS_PROVIDERS_INIT: Once = Once::new();
    RESOURCES_INIT.call_once(|| {
        gio::resources_register_include!("recall.gresource")
            .expect("failed to register embedded resources");
    });

    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    CSS_PROVIDERS_INIT.call_once(|| {
        let icon_theme = gtk::IconTheme::for_display(&display);
        icon_theme.add_resource_path("/io/github/basshift/Recall/icons/hicolor");
        icon_theme.add_resource_path("/io/github/basshift/Recall/icons");

        for resource_path in [
            "/io/github/basshift/Recall/style.vars.css",
            "/io/github/basshift/Recall/style.css",
            "/io/github/basshift/Recall/style.light.css",
            "/io/github/basshift/Recall/style.dark.css",
            "/io/github/basshift/Recall/style.mobile.css",
        ] {
            let provider = gtk::CssProvider::new();
            provider.load_from_resource(resource_path);
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
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

    let icon = gtk::Image::from_icon_name("io.github.basshift.Recall");
    icon.set_pixel_size(168);
    icon.add_css_class("main-menu-icon");

    let title = gtk::Label::new(Some(&tr("Recall")));
    title.add_css_class("main-menu-title");
    title.add_css_class("title-1");

    let buttons_box = gtk::Box::new(gtk::Orientation::Vertical, 13);
    buttons_box.set_halign(gtk::Align::Center);
    buttons_box.add_css_class("main-menu-actions");

    let continue_button = gtk::Button::new();
    continue_button.add_css_class("main-menu-button");
    continue_button.set_size_request(210, 40);
    continue_button.set_halign(gtk::Align::Center);
    let saved_run = session_save::load_saved_run();
    set_continue_button_content(&continue_button, saved_run.as_ref());
    continue_button.set_visible(saved_run.is_some());
    continue_button.connect_clicked({
        let state = state.clone();
        move |_| {
            continue_last_run(&state);
        }
    });

    let new_button = gtk::Button::new();
    new_button.add_css_class("main-menu-button-primary");
    new_button.add_css_class("suggested-action");
    new_button.set_size_request(210, 40);
    new_button.set_halign(gtk::Align::Center);
    let new_button_label = gtk::Label::new(Some(&tr("New Game")));
    new_button_label.add_css_class("main-menu-button-label");
    new_button.set_child(Some(&new_button_label));
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
    let board_hover_state = state.clone();
    let board_motion = gtk::EventControllerMotion::new();
    board_motion.connect_enter(move |_, _, _| {
        let st = board_hover_state.borrow();
        if !is_game_view_active(&st) || st.lock_input {
            return;
        }
        if let Some(container) = &st.board_container {
            container.remove_css_class("no-hover");
        }
    });
    let board_leave_state = state.clone();
    board_motion.connect_leave(move |_| {
        let st = board_leave_state.borrow();
        if let Some(container) = &st.board_container {
            container.add_css_class("no-hover");
        }
    });
    board_card.add_controller(board_motion);

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
        st.board_shell = Some(board_frame.clone());
    }

    root
}

fn spawn_victory_confetti_piece(layer: &gtk::Fixed, x: f64, y: f64) {
    let color_idx = glib::random_int_range(0, 6);
    let shape_symbol = match glib::random_int_range(0, 3) {
        0 => "■",
        1 => "◆",
        _ => "●",
    };
    let shape_class = match glib::random_int_range(0, 3) {
        0 => "shape-square",
        1 => "shape-diamond",
        _ => "shape-circle",
    };
    let drift_class = if glib::random_int_range(0, 2) == 0 {
        "drift-left"
    } else {
        "drift-right"
    };
    let speed_class = match glib::random_int_range(0, 3) {
        0 => "speed-a",
        1 => "speed-b",
        _ => "speed-c",
    };
    let particle = gtk::Label::builder()
        .label(shape_symbol)
        .css_classes(vec![
            "victory-confetti-particle",
            &format!("color-{}", color_idx),
            shape_class,
            drift_class,
            speed_class,
        ])
        .build();

    particle.set_can_target(false);
    layer.put(&particle, x, y);

    glib::timeout_add_local_once(std::time::Duration::from_millis(1800), {
        let layer_weak = layer.downgrade();
        let particle_weak = particle.downgrade();
        move || {
            if let (Some(layer), Some(particle)) = (layer_weak.upgrade(), particle_weak.upgrade()) {
                layer.remove(&particle);
            }
        }
    });
}

fn random_confetti_spawn_x(layer: &gtk::Fixed) -> f64 {
    let layer_width = layer.width().max(280) as f64;
    let side_padding = 8.0;
    let min_x = side_padding;
    let max_x = (layer_width - side_padding - 12.0).max(min_x + 1.0);
    glib::random_double_range(min_x, max_x)
}

fn remove_source_id_if_active(source_id: glib::SourceId) {
    // Some timers may already be removed by GLib after returning Break.
    // Removing a missing source with SourceId::remove() panics in this glib version.
    unsafe {
        let _ = glib::ffi::g_source_remove(source_id.as_raw());
    }
}

pub(super) fn stop_victory_sparks(st: &mut AppState) {
    if let Some(handle) = st.spark_timer_handle.take() {
        remove_source_id_if_active(handle);
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
    let mut elapsed_ms = 0u32;
    let handle = glib::timeout_add_local(std::time::Duration::from_millis(85), move || {
        let Some(state) = state_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };

        let in_victory_view = {
            let st = state.borrow();
            st.view_stack
                .as_ref()
                .and_then(|stack| stack.visible_child_name())
                .as_deref()
                == Some("victory")
        };
        if !in_victory_view {
            state.borrow_mut().spark_timer_handle = None;
            return glib::ControlFlow::Break;
        }

        elapsed_ms = elapsed_ms.saturating_add(85);
        if elapsed_ms >= 3500 {
            state.borrow_mut().spark_timer_handle = None;
            return glib::ControlFlow::Break;
        }

        if let Some(layer) = &layer {
            let spawn_count = glib::random_int_range(1, 4);
            for _ in 0..spawn_count {
                let x = random_confetti_spawn_x(layer);
                let y = glib::random_double_range(-24.0, -5.0);
                spawn_victory_confetti_piece(layer, x, y);
            }
        } else {
            state.borrow_mut().spark_timer_handle = None;
            return glib::ControlFlow::Break;
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

    let rank_art = gtk::Image::from_resource("/io/github/basshift/Recall/victory/rank-c.svg");
    rank_art.add_css_class("victory-rank-art");
    rank_art.set_pixel_size(160);
    rank_art.set_halign(gtk::Align::Center);

    let title = gtk::Label::new(Some(&tr("Well done!")));
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

    let again_btn = gtk::Button::with_label(&tr("Play Again"));
    again_btn.add_css_class("suggested-action");
    let menu_btn = gtk::Button::with_label(&tr("Main Menu"));

    again_btn.connect_clicked({
        let state = state.clone();
        move |_| {
            restart_game(&state);
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
        st.victory_art_resource = None;
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
    st.grid_buttons[index].add_css_class("active");
    play_flip_show(&mut st, index);
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
                let (mismatch_pause_ms, penalty_plan) = if st.difficulty == Difficulty::Trio {
                (
                    trio_penalties::mismatch_pause_ms(st.trio_level),
                    trio_penalties::register_mismatch_and_plan_reshuffle(&mut st, first_pick_index),
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
            drop(st);
            clear_keyboard_focus(state);
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
            let mut st = state.borrow_mut();
            mark_run_dirty(&mut st);
        }
        FlipOutcome::CompleteMatch => {
            st.run_matches = st.run_matches.saturating_add(1);
            if st.difficulty == Difficulty::Trio {
                trio_penalties::reset_penalty_after_match(&mut st);
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
            clear_keyboard_focus(state);
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
        Difficulty::Easy => 4.0,
        Difficulty::Medium => 7.0,
        Difficulty::Hard => 10.0,
        Difficulty::Impossible => classic_penalties::PREVIEW_SECONDS,
        Difficulty::Trio => match st.trio_level {
            1 => 9.0,
            2 => 11.0,
            3 => 14.0,
            _ => 15.0,
        },
        Difficulty::Infinite => match infinite::classic_difficulty_for_round(st.infinite_round) {
            Difficulty::Easy => 4.0,
            Difficulty::Medium => 7.0,
            Difficulty::Hard => 10.0,
            Difficulty::Impossible => classic_penalties::PREVIEW_SECONDS,
            _ => 4.0,
        },
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
            container.add_css_class("no-hover");
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
            button.remove_css_class("matched-dim");
            button.remove_css_class("active");
            button.remove_css_class("match-bump");
            button.remove_css_class("mismatch-shake");
            clear_flip_classes(button);
            if let Some(child) = button.child() {
                child.queue_draw();
            }
        }
    }
    clear_keyboard_focus(state);

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
        stop_timer(&mut st);
        stop_preview(&mut st);
        stop_victory_sparks(&mut st);
        st.game_id = st.game_id.wrapping_add(1);
        st.lock_input = false;
        st.flipped_indices.clear();
        if infinite::is_infinite(st.difficulty) {
            infinite::prepare_start(&mut st);
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
            finalize_infinite_run_if_needed(&mut st);
            st.active_session_started = false;
            clear_saved_run_and_refresh(&mut st);
        }
        if st.difficulty != difficulty {
            finalize_infinite_run_if_needed(&mut st);
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

pub(super) fn apply_trio_level_change(state: &Rc<RefCell<AppState>>, level: u8) {
    let should_refresh = {
        let mut st = state.borrow_mut();
        if st.trio_level == level.clamp(1, 4) {
            false
        } else {
            st.set_trio_level(level);
            st.difficulty == Difficulty::Trio
        }
    };

    if should_refresh {
        rebuild_board(state);
        show_game(state);
    }
}
