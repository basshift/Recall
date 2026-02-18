use std::cell::RefCell;
use std::rc::Rc;
use gtk4::glib;
use gtk4::gdk;
use gtk4::prelude::*;
use super::state::{AppState, Difficulty, TileStatus};
use super::hud::update_subtitle;
use super::scene::rebuild_board;
use super::app::{apply_difficulty_change, apply_tri_level_change, show_game};
use super::hud::stop_preview;
use super::hud::stop_timer;
use super::app::clear_flip_classes;
use super::app::redraw_button_child;
use super::infinite;

enum NearWinResult {
    Applied(usize),
    NoTiles,
    NoGroupFound,
}

pub fn debug_mode_enabled() -> bool {
    match std::env::var("RECALL_DEBUG") {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

pub fn handle_debug_shortcut(
    state: &Rc<RefCell<AppState>>,
    key: gdk::Key,
    mods: gdk::ModifierType,
) -> bool {
    let want_ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
    if !want_ctrl {
        return false;
    }

    let is_debug_key = matches!(
        key,
        gdk::Key::N
            | gdk::Key::n
            | gdk::Key::R
            | gdk::Key::r
            | gdk::Key::_1
            | gdk::Key::KP_1
            | gdk::Key::_2
            | gdk::Key::KP_2
            | gdk::Key::_3
            | gdk::Key::KP_3
            | gdk::Key::_4
            | gdk::Key::KP_4
            | gdk::Key::F9
    );
    if !is_debug_key {
        return false;
    }

    if !debug_mode_enabled() {
        show_debug_banner(state, "DEBUG OFF | export RECALL_DEBUG=1");
        return true;
    }

    match key {
        gdk::Key::N | gdk::Key::n | gdk::Key::F9 => {
            match debug_prepare_near_win(state) {
                NearWinResult::Applied(remaining) => {
                    let st = state.borrow();
                    eprintln!(
                        "[DEBUG][{}] Board prepared: one group left ({} cards).",
                        st.difficulty.name(),
                        remaining
                    );
                    show_debug_banner(state, &format!("DEBUG | Near-win ready ({remaining})"));
                    true
                }
                NearWinResult::NoTiles => {
                    let st = state.borrow();
                    eprintln!(
                        "[DEBUG][{}] Near-win skipped: board has no tiles yet.",
                        st.difficulty.name()
                    );
                    show_debug_banner(state, "DEBUG | Near-win failed (no tiles)");
                    true
                }
                NearWinResult::NoGroupFound => {
                    let st = state.borrow();
                    eprintln!(
                        "[DEBUG][{}] Near-win failed: no group with match_size={} found.",
                        st.difficulty.name(),
                        st.match_size
                    );
                    show_debug_banner(state, "DEBUG | Near-win failed (no group)");
                    true
                }
            }
        }
        gdk::Key::R | gdk::Key::r => {
            let is_infinite_mode = {
                let st = state.borrow();
                infinite::is_infinite(st.difficulty)
            };
            if is_infinite_mode {
                {
                    let mut st = state.borrow_mut();
                    let level_up = infinite::advance_round(&mut st);
                    if let Some(level_up) = level_up {
                        eprintln!(
                            "[DEBUG][Infinite] Forced next round -> round {} (level up: {} -> {})",
                            st.infinite_round,
                            infinite::level_name(level_up.from_level),
                            infinite::level_name(level_up.to_level)
                        );
                    } else {
                        eprintln!(
                            "[DEBUG][Infinite] Forced next round -> round {} (level {})",
                            st.infinite_round, st.recall_level
                        );
                    }
                }
                show_game(state);
                show_debug_banner(state, "DEBUG | Infinite next round");
                true
            } else {
                let mut st = state.borrow_mut();
                let mode_name = st.difficulty.name();
                st.active_session_started = false;
                drop(st);
                show_game(state);
                eprintln!("[DEBUG][{}] Restarted current map.", mode_name);
                show_debug_banner(state, "DEBUG | Map restarted");
                true
            }
        }
        gdk::Key::_1 | gdk::Key::KP_1 => {
            debug_force_level(state, 1)
        }
        gdk::Key::_2 | gdk::Key::KP_2 => {
            debug_force_level(state, 2)
        }
        gdk::Key::_3 | gdk::Key::KP_3 => {
            debug_force_level(state, 3)
        }
        gdk::Key::_4 | gdk::Key::KP_4 => {
            debug_force_level(state, 4)
        }
        _ => false,
    }
}

fn debug_force_level(state: &Rc<RefCell<AppState>>, level: u8) -> bool {
    let mut st = state.borrow_mut();
    if infinite::is_infinite(st.difficulty) {
        st.set_recall_level(level.clamp(1, 4));
        let level_name = infinite::level_name(st.recall_level).to_string();
        eprintln!(
            "[DEBUG][Infinite] Forced level -> {} (round {})",
            level_name,
            st.infinite_round
        );
        drop(st);
        rebuild_board(state);
        show_game(state);
        show_debug_banner(state, &format!("DEBUG | Infinite {}", level_name));
        return true;
    }

    if st.difficulty == Difficulty::Tri {
        let tri_level = level.clamp(1, 4);
        drop(st);
        apply_tri_level_change(state, tri_level);
        eprintln!(
            "[DEBUG][Tri] Forced level -> {}",
            tri_level
        );
        show_debug_banner(state, &format!("DEBUG | Tri level {}", tri_level));
        return true;
    }

    let target = match level.clamp(1, 4) {
        1 => Difficulty::Easy,
        2 => Difficulty::Medium,
        3 => Difficulty::Hard,
        _ => Difficulty::Impossible,
    };
    drop(st);
    apply_difficulty_change(state, target);
    eprintln!("[DEBUG][Classic] Forced difficulty -> {}", target.name());
    show_debug_banner(state, &format!("DEBUG | Classic {}", target.name()));
    true
}

fn debug_prepare_near_win(state: &Rc<RefCell<AppState>>) -> NearWinResult {
    let mut st = state.borrow_mut();
    if st.tiles.is_empty() {
        return NearWinResult::NoTiles;
    }

    use std::collections::HashMap;
    let mut by_value: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, tile) in st.tiles.iter().enumerate() {
        if tile.value.is_empty() {
            continue;
        }
        by_value.entry(tile.value.clone()).or_default().push(idx);
    }

    let match_size = st.match_size.max(2);
    let Some(remaining_group) = by_value
        .values()
        .find(|indices| indices.len() >= match_size)
        .map(|indices| indices.iter().take(match_size).copied().collect::<Vec<usize>>())
    else {
        return NearWinResult::NoGroupFound;
    };

    stop_preview(&mut st);
    stop_timer(&mut st);
    st.lock_input = false;
    st.flipped_indices.clear();

    for idx in 0..st.tiles.len() {
        let keep_hidden = remaining_group.contains(&idx);
        if st.tiles[idx].value.is_empty() {
            st.tiles[idx].status = TileStatus::Matched;
        } else if keep_hidden {
            st.tiles[idx].status = TileStatus::Hidden;
        } else {
            st.tiles[idx].status = TileStatus::Matched;
        }

        if let Some(button) = st.grid_buttons.get(idx) {
            clear_flip_classes(button);
            button.remove_css_class("active");
            button.remove_css_class("mismatch-shake");
            button.remove_css_class("match-bump");
            if keep_hidden {
                button.remove_css_class("matched");
            } else {
                button.add_css_class("matched");
            }
            redraw_button_child(button);
        }
    }

    update_subtitle(&st);
    NearWinResult::Applied(remaining_group.len())
}

fn show_debug_banner(state: &Rc<RefCell<AppState>>, message: &str) {
    let message = message.to_string();
    let game_id = {
        let st = state.borrow();
        if let Some(subtitle) = &st.title_game_subtitle {
            subtitle.set_text(&message);
        }
        st.game_id
    };
    let state_weak = Rc::downgrade(state);
    glib::timeout_add_local_once(std::time::Duration::from_millis(1200), move || {
        if let Some(state) = state_weak.upgrade() {
            let st = state.borrow();
            if st.game_id == game_id {
                update_subtitle(&st);
            }
        }
    });
}
