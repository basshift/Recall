use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use super::app::{apply_difficulty_change, apply_tri_level_change};
use super::classic::{difficulty_from_level, CLASSIC_LEVEL_OPTIONS};
use super::state::{AppState, Difficulty};

fn add_mode_row(content: &gtk::Box, label: &str, on_select: impl Fn() + 'static) {
    let main_button = gtk::Button::with_label(label);
    main_button.set_hexpand(true);
    main_button.set_size_request(-1, 42);
    main_button.add_css_class("mode-dialog-button");
    main_button.connect_clicked(move |_| on_select());
    content.append(&main_button);
}

fn show_difficulty_dialog(state: &Rc<RefCell<AppState>>, app: &adw::Application, is_tri: bool) {
    let parent_window = app.active_window();
    let dialog = adw::Dialog::new();
    dialog.set_can_close(true);

    let title = gtk::Label::new(Some("Choose difficulty"));
    title.add_css_class("dialog-header-title");
    title.set_halign(gtk::Align::Center);

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));
    header.set_show_end_title_buttons(true);
    header.add_css_class("flat");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.add_css_class("mode-dialog-content");
    content.set_hexpand(true);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let difficulty_options: Vec<(&str, u8)> = if is_tri {
        vec![("Easy", 1), ("Normal", 2), ("Hard", 3), ("Expert", 4)]
    } else {
        CLASSIC_LEVEL_OPTIONS.to_vec()
    };

    for (label, level) in difficulty_options {
        let button = gtk::Button::with_label(label);
        button.set_hexpand(true);
        button.set_size_request(-1, 42);
        button.add_css_class("mode-dialog-button");
        button.connect_clicked({
            let state = state.clone();
            let dialog = dialog.clone();
            move |_| {
                if is_tri {
                    apply_tri_level_change(&state, level);
                    apply_difficulty_change(&state, Difficulty::Tri);
                } else {
                    apply_difficulty_change(&state, difficulty_from_level(level));
                }
                dialog.close();
            }
        });
        content.append(&button);
    }

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content));

    dialog.set_child(Some(&toolbar));
    dialog.present(parent_window.as_ref());
}

pub fn show_mode_dialog(state: &Rc<RefCell<AppState>>, app: &adw::Application) {
    let parent_window = app.active_window();
    let dialog = adw::Dialog::new();
    dialog.set_can_close(true);

    let title = gtk::Label::new(Some("Choose mode"));
    title.add_css_class("dialog-header-title");
    title.set_halign(gtk::Align::Center);

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));
    header.set_show_end_title_buttons(true);
    header.add_css_class("flat");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.add_css_class("mode-dialog-content");
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    add_mode_row(
        &content,
        "Classic",
        {
            let state = state.clone();
            let app = app.clone();
            let dialog = dialog.clone();
            move || {
                dialog.close();
                show_difficulty_dialog(&state, &app, false);
            }
        },
    );

    add_mode_row(
        &content,
        "Tri",
        {
            let state = state.clone();
            let app = app.clone();
            let dialog = dialog.clone();
            move || {
                dialog.close();
                show_difficulty_dialog(&state, &app, true);
            }
        },
    );

    add_mode_row(
        &content,
        "Infinite",
        {
            let state = state.clone();
            let dialog = dialog.clone();
            move || {
                dialog.close();
                apply_difficulty_change(&state, Difficulty::RecallMode);
            }
        },
    );

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content));

    dialog.set_child(Some(&toolbar));
    dialog.present(parent_window.as_ref());
}
