use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use crate::i18n::tr;

use super::app::{apply_difficulty_change, apply_trio_level_change};
use super::classic::{difficulty_from_level, CLASSIC_LEVEL_OPTIONS};
use super::state::{AppState, Difficulty};

fn difficulty_title(level: u8) -> String {
    match level {
        1 => tr("Easy"),
        2 => tr("Medium"),
        3 => tr("Hard"),
        _ => tr("Expert"),
    }
}

fn difficulty_grid_size(level: u8, is_trio: bool) -> &'static str {
    if is_trio {
        match level {
            1 => "4x6",
            2 => "5x6",
            3 => "6x7",
            _ => "6x8",
        }
    } else {
        match level {
            1 => "3x4",
            2 => "4x6",
            3 => "6x7",
            _ => "6x8",
        }
    }
}

fn build_page_header(show_back_button: bool) -> adw::HeaderBar {
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(true);
    header.set_show_back_button(show_back_button);
    header.add_css_class("flat");
    header
}

fn build_single_row_list(row: &adw::ActionRow) -> gtk::ListBox {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();
    list.add_css_class("boxed-list");
    list.append(row);
    list
}

fn build_mode_row(
    title: &str,
    subtitle: &str,
    show_chevron: bool,
    on_select: impl Fn() + 'static,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .activatable(true)
        .build();
    row.add_css_class("mode-native-row");

    if show_chevron {
        let chevron = gtk::Image::from_icon_name("go-next-symbolic");
        row.add_suffix(&chevron);
    }

    row.connect_activated(move |_| on_select());
    row
}

fn build_difficulty_row(
    level: u8,
    is_trio: bool,
    on_select: impl Fn() + 'static,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(difficulty_title(level))
        .activatable(true)
        .build();
    row.add_css_class("difficulty-native-row");

    let grid_size = gtk::Label::new(Some(difficulty_grid_size(level, is_trio)));
    grid_size.add_css_class("dim-label");
    grid_size.add_css_class("caption");
    row.add_suffix(&grid_size);

    row.connect_activated(move |_| on_select());
    row
}

fn build_mode_content(
    navigation_view: &adw::NavigationView,
    classic_difficulty_page: &adw::NavigationPage,
    trio_difficulty_page: &adw::NavigationPage,
    state: &Rc<RefCell<AppState>>,
    dialog: &adw::Dialog,
) -> adw::Clamp {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);

    let classic_row = build_mode_row(
        &tr("Classic"),
        &tr("Match identical cards and clear the full board"),
        true,
        {
            let navigation_view = navigation_view.clone();
            let target_page = classic_difficulty_page.clone();
            move || navigation_view.push(&target_page)
        },
    );
    let classic_list = build_single_row_list(&classic_row);
    content.append(&classic_list);

    let trio_row = build_mode_row(
        &tr("Trio"),
        &tr("Form 3-card groups and clear each stage"),
        true,
        {
            let navigation_view = navigation_view.clone();
            let target_page = trio_difficulty_page.clone();
            move || navigation_view.push(&target_page)
        },
    );
    let trio_list = build_single_row_list(&trio_row);
    content.append(&trio_list);

    let infinite_row = build_mode_row(
        &tr("Infinite"),
        &tr("Classic core rules with endless progression"),
        false,
        {
            let state = state.clone();
            let dialog = dialog.clone();
            move || {
                apply_difficulty_change(&state, Difficulty::Infinite);
                dialog.close();
            }
        },
    );
    let infinite_list = build_single_row_list(&infinite_row);
    content.append(&infinite_list);

    let clamp = adw::Clamp::builder().maximum_size(520).build();
    clamp.set_margin_top(12);
    clamp.set_margin_bottom(0);
    clamp.set_margin_start(15);
    clamp.set_margin_end(15);
    clamp.set_child(Some(&content));
    clamp
}

fn build_difficulty_content(
    state: &Rc<RefCell<AppState>>,
    dialog: &adw::Dialog,
    options: &[u8],
    is_trio: bool,
) -> adw::Clamp {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);

    for &level in options {
        let row = build_difficulty_row(level, is_trio, {
            let state = state.clone();
            let dialog = dialog.clone();
            move || {
                if is_trio {
                    let is_current_trio = state.borrow().difficulty == Difficulty::Trio;
                    apply_trio_level_change(&state, level);
                    if !is_current_trio {
                        apply_difficulty_change(&state, Difficulty::Trio);
                    }
                } else {
                    apply_difficulty_change(&state, difficulty_from_level(level));
                }
                dialog.close();
            }
        });
        let list = build_single_row_list(&row);
        content.append(&list);
    }

    let clamp = adw::Clamp::builder().maximum_size(520).build();
    clamp.set_margin_top(12);
    clamp.set_margin_bottom(0);
    clamp.set_margin_start(15);
    clamp.set_margin_end(15);
    clamp.set_child(Some(&content));
    clamp
}

fn build_difficulty_page(
    state: &Rc<RefCell<AppState>>,
    dialog: &adw::Dialog,
    title_text: &str,
    options: &[u8],
    is_trio: bool,
) -> adw::NavigationPage {
    let header = build_page_header(true);
    let content = build_difficulty_content(state, dialog, options, is_trio);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content));

    adw::NavigationPage::builder()
        .title(title_text)
        .child(&toolbar)
        .build()
}

pub fn show_mode_dialog(state: &Rc<RefCell<AppState>>, app: &adw::Application) {
    let parent_window = app.active_window();
    let dialog = adw::Dialog::new();
    dialog.set_can_close(true);

    let compact_layout = if let Some(window) = parent_window.as_ref() {
        let width = window.allocated_width().max(1);
        let height = window.allocated_height().max(1);
        (width < 760 && height < 620) || width < 520
    } else {
        state.borrow().compact_layout
    };

    dialog.set_follows_content_size(false);
    let (content_width, content_height) = parent_window
        .as_ref()
        .map(|window| {
            let width = window.width().max(window.allocated_width()).max(1);
            if compact_layout {
                ((width - 20).clamp(300, 380), 308)
            } else {
                ((width - 32).clamp(320, 420), 316)
            }
        })
        .unwrap_or((420, 316));
    dialog.set_content_width(content_width);
    dialog.set_content_height(content_height);

    let navigation_view = adw::NavigationView::new();
    navigation_view.set_hexpand(true);
    navigation_view.set_vexpand(true);

    let classic_difficulty_page = build_difficulty_page(
        state,
        &dialog,
        &tr("Classic Difficulty"),
        &CLASSIC_LEVEL_OPTIONS,
        false,
    );
    let trio_difficulty_page = build_difficulty_page(
        state,
        &dialog,
        &tr("Trio Difficulty"),
        &[1, 2, 3, 4],
        true,
    );

    let mode_header = build_page_header(false);
    let mode_content = build_mode_content(
        &navigation_view,
        &classic_difficulty_page,
        &trio_difficulty_page,
        state,
        &dialog,
    );

    let mode_toolbar = adw::ToolbarView::new();
    mode_toolbar.add_top_bar(&mode_header);
    mode_toolbar.set_content(Some(&mode_content));

    let mode_page = adw::NavigationPage::builder()
        .title(tr("Choose Mode"))
        .child(&mode_toolbar)
        .build();

    navigation_view.add(&mode_page);
    navigation_view.add(&classic_difficulty_page);
    navigation_view.add(&trio_difficulty_page);

    dialog.set_child(Some(&navigation_view));
    dialog.present(parent_window.as_ref());
}
