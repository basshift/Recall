use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;

pub fn show_instructions_dialog(app: &adw::Application) -> adw::AlertDialog {
    let dialog = adw::AlertDialog::new(
        Some("Instructions"),
        Some(
            "Memorize the board and find matching pairs.\n\
Reveal cards to discover symbols.\n\
Complete the board with the best accuracy and time you can.",
        ),
    );
    dialog.add_response("ok", "Got it");
    dialog.set_default_response(Some("ok"));
    dialog.set_close_response("ok");
    dialog.present(app.active_window().as_ref());
    dialog
}

pub fn show_about_dialog(app: &adw::Application) -> adw::AboutDialog {
    let dialog = adw::AboutDialog::builder()
        .application_name("Recall")
        .application_icon("io.github.basshift.recall")
        .developer_name("Sebastian Dávila (Basshift)")
        .developers(vec!["Sebastian Dávila (Basshift)"])
        .version("0.1.0")
        .comments("A memory game for finding pairs.")
        .issue_url("https://github.com/basshift/Recall/issues")
        .support_url("https://github.com/basshift/Recall")
        .website("https://github.com/basshift/Recall")
        .build();
    dialog.add_legal_section(
        "Recall",
        Some("© 2026 Sebastian Dávila (Basshift)"),
        gtk::License::MitX11,
        None,
    );
    dialog.present(app.active_window().as_ref());
    dialog
}
