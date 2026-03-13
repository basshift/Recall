use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;

use crate::i18n::tr;

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn show_instructions_dialog(app: &adw::Application) -> adw::Dialog {
    let dialog_title = xml_escape(&tr("How to Play"));
    let slide_1_title = xml_escape(&tr("Read, Then Recall"));
    let slide_1_desc = xml_escape(&tr(
        "Brief reveal first. Keep mental anchors, then match from memory",
    ));
    let slide_2_title = xml_escape(&tr("Match the Full Board"));
    let slide_2_desc = xml_escape(&tr(
        "Clear every card to win. Build a steady rhythm and protect your accuracy",
    ));
    let slide_3_title = xml_escape(&tr("Know the Modes"));
    let slide_3_desc = xml_escape(&tr(
        "Classic matches pairs, Trio matches three, and Infinite escalates pace every round",
    ));
    let slide_4_title = xml_escape(&tr("Challenge Scale"));
    let slide_4_desc = xml_escape(&tr(
        "Pick your level: bigger boards, less time to memorize",
    ));
    let slide_5_title = xml_escape(&tr("Restless Board"));
    let slide_5_desc = xml_escape(&tr(
        "Stumble several times and hidden cards may reshuffle",
    ));
    let prev_tip = xml_escape(&tr("Previous"));
    let next_tip = xml_escape(&tr("Next"));

    let xml = format!(
        r#"
<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="AdwDialog" id="how_to_play_dialog">
    <property name="title">{dialog_title}</property>
    <property name="can-close">True</property>
    <property name="content-width">600</property>
    <property name="content-height">500</property>
    <property name="css-classes">how-to-play-dialog</property>
    <child>
      <object class="AdwToolbarView">
        <child type="top">
          <object class="AdwHeaderBar">
            <property name="show-end-title-buttons">True</property>
            <property name="title-widget">
              <object class="AdwCarouselIndicatorDots">
                <property name="carousel">how_to_play_carousel</property>
              </object>
            </property>
          </object>
        </child>
        <child>
          <object class="GtkOverlay">
            <child>
              <object class="AdwClamp">
                <property name="maximum-size">600</property>
                <property name="tightening-threshold">400</property>
                <child>
                  <object class="GtkBox">
                    <property name="halign">center</property>
                    <property name="vexpand">True</property>
                    <child>
                      <object class="AdwCarousel" id="how_to_play_carousel">
                        <property name="allow-long-swipes">True</property>
                        <property name="allow-scroll-wheel">True</property>
                        <property name="spacing">36</property>
                        <child>
                          <object class="GtkBox">
                            <property name="orientation">vertical</property>
                            <property name="hexpand">True</property>
                            <property name="vexpand">True</property>
                            <child>
                              <object class="GtkAspectFrame">
                                <property name="halign">center</property>
                                <property name="valign">center</property>
                                <property name="hexpand">True</property>
                                <property name="vexpand">True</property>
                                <property name="ratio">1</property>
                                <child>
                                  <object class="GtkPicture">
                                    <property name="file">resource://io/github/basshift/Recall/howto/01-flow.svg</property>
                                  </object>
                                </child>
                              </object>
                            </child>
                            <child>
                              <object class="AdwStatusPage">
                                <property name="title">{slide_1_title}</property>
                                <property name="description">{slide_1_desc}</property>
                                <property name="can-focus">true</property>
                              </object>
                            </child>
                          </object>
                        </child>
                        <child>
                          <object class="GtkBox">
                            <property name="orientation">vertical</property>
                            <property name="hexpand">True</property>
                            <property name="vexpand">True</property>
                            <child>
                              <object class="GtkAspectFrame">
                                <property name="halign">center</property>
                                <property name="valign">center</property>
                                <property name="hexpand">True</property>
                                <property name="vexpand">True</property>
                                <property name="ratio">1</property>
                                <child>
                                  <object class="GtkPicture">
                                    <property name="file">resource://io/github/basshift/Recall/howto/02-goal.svg</property>
                                  </object>
                                </child>
                              </object>
                            </child>
                            <child>
                              <object class="AdwStatusPage">
                                <property name="title">{slide_2_title}</property>
                                <property name="description">{slide_2_desc}</property>
                                <property name="can-focus">true</property>
                              </object>
                            </child>
                          </object>
                        </child>
                        <child>
                          <object class="GtkBox">
                            <property name="orientation">vertical</property>
                            <property name="hexpand">True</property>
                            <property name="vexpand">True</property>
                            <child>
                              <object class="GtkAspectFrame">
                                <property name="halign">center</property>
                                <property name="valign">center</property>
                                <property name="hexpand">True</property>
                                <property name="vexpand">True</property>
                                <property name="ratio">1</property>
                                <child>
                                  <object class="GtkPicture">
                                    <property name="file">resource://io/github/basshift/Recall/howto/03-modes.svg</property>
                                  </object>
                                </child>
                              </object>
                            </child>
                            <child>
                              <object class="AdwStatusPage">
                                <property name="title">{slide_3_title}</property>
                                <property name="description">{slide_3_desc}</property>
                                <property name="can-focus">true</property>
                              </object>
                            </child>
                          </object>
                        </child>
                        <child>
                          <object class="GtkBox">
                            <property name="orientation">vertical</property>
                            <property name="hexpand">True</property>
                            <property name="vexpand">True</property>
                            <child>
                              <object class="GtkAspectFrame">
                                <property name="halign">center</property>
                                <property name="valign">center</property>
                                <property name="hexpand">True</property>
                                <property name="vexpand">True</property>
                                <property name="ratio">1</property>
                                <child>
                                  <object class="GtkPicture">
                                    <property name="file">resource://io/github/basshift/Recall/howto/04-difficulty.svg</property>
                                  </object>
                                </child>
                              </object>
                            </child>
                            <child>
                              <object class="AdwStatusPage">
                                <property name="title">{slide_4_title}</property>
                                <property name="description">{slide_4_desc}</property>
                                <property name="can-focus">true</property>
                              </object>
                            </child>
                          </object>
                        </child>
                        <child>
                          <object class="GtkBox">
                            <property name="orientation">vertical</property>
                            <property name="hexpand">True</property>
                            <property name="vexpand">True</property>
                            <child>
                              <object class="GtkAspectFrame">
                                <property name="halign">center</property>
                                <property name="valign">center</property>
                                <property name="hexpand">True</property>
                                <property name="vexpand">True</property>
                                <property name="ratio">1</property>
                                <child>
                                  <object class="GtkPicture">
                                    <property name="file">resource://io/github/basshift/Recall/howto/05-restless.svg</property>
                                  </object>
                                </child>
                              </object>
                            </child>
                            <child>
                              <object class="AdwStatusPage">
                                <property name="title">{slide_5_title}</property>
                                <property name="description">{slide_5_desc}</property>
                                <property name="can-focus">true</property>
                              </object>
                            </child>
                          </object>
                        </child>
                      </object>
                    </child>
                  </object>
                </child>
              </object>
            </child>
            <child type="overlay">
              <object class="GtkButton" id="how_to_play_prev">
                <property name="halign">start</property>
                <property name="valign">center</property>
                <property name="margin-start">16</property>
                <property name="icon-name">go-previous-symbolic</property>
                <property name="tooltip-text">{prev_tip}</property>
                <property name="sensitive">False</property>
                <style>
                  <class name="circular"/>
                </style>
              </object>
            </child>
            <child type="overlay">
              <object class="GtkButton" id="how_to_play_next">
                <property name="halign">end</property>
                <property name="valign">center</property>
                <property name="margin-end">16</property>
                <property name="icon-name">go-next-symbolic</property>
                <property name="tooltip-text">{next_tip}</property>
                <style>
                  <class name="circular"/>
                </style>
              </object>
            </child>
          </object>
        </child>
      </object>
    </child>
  </object>
</interface>
"#,
    );

    let builder = gtk::Builder::from_string(&xml);
    let dialog = builder
        .object::<adw::Dialog>("how_to_play_dialog")
        .expect("failed to build how-to-play dialog");
    let carousel = builder
        .object::<adw::Carousel>("how_to_play_carousel")
        .expect("failed to get how-to-play carousel");
    let prev = builder
        .object::<gtk::Button>("how_to_play_prev")
        .expect("failed to get how-to-play prev button");
    let next = builder
        .object::<gtk::Button>("how_to_play_next")
        .expect("failed to get how-to-play next button");

    let update_buttons = {
        let carousel = carousel.clone();
        let prev = prev.clone();
        let next = next.clone();
        move || {
            let current_page = carousel.position().round() as u32;
            let total_pages = carousel.n_pages();
            prev.set_sensitive(current_page > 0);
            next.set_sensitive(current_page + 1 < total_pages);
        }
    };
    update_buttons();

    prev.connect_clicked({
        let carousel = carousel.clone();
        let update_buttons = update_buttons.clone();
        move |_| {
            let current_page = carousel.position().round() as u32;
            if current_page > 0 {
                let page = carousel.nth_page(current_page - 1);
                carousel.scroll_to(&page, true);
            }
            update_buttons();
        }
    });
    next.connect_clicked({
        let carousel = carousel.clone();
        let update_buttons = update_buttons.clone();
        move |_| {
            let current_page = carousel.position().round() as u32;
            let total_pages = carousel.n_pages();
            if current_page + 1 < total_pages {
                let page = carousel.nth_page(current_page + 1);
                carousel.scroll_to(&page, true);
            }
            update_buttons();
        }
    });

    carousel.connect_page_changed({
        let update_buttons = update_buttons.clone();
        move |_, _| update_buttons()
    });

    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed({
        let carousel = carousel.clone();
        move |_, key, _, _| {
            let current_page = carousel.position().round() as u32;
            let total_pages = carousel.n_pages();
            match key {
                gtk::gdk::Key::Left => {
                    if current_page > 0 {
                        let page = carousel.nth_page(current_page - 1);
                        carousel.scroll_to(&page, true);
                        return glib::Propagation::Stop;
                    }
                }
                gtk::gdk::Key::Right => {
                    if current_page + 1 < total_pages {
                        let page = carousel.nth_page(current_page + 1);
                        carousel.scroll_to(&page, true);
                        return glib::Propagation::Stop;
                    }
                }
                _ => {}
            }
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(key_controller);

    dialog.present(app.active_window().as_ref());
    dialog
}

pub fn show_about_dialog(app: &adw::Application) -> adw::AboutDialog {
    let dialog = adw::AboutDialog::builder()
        .application_name("Recall")
        .application_icon("io.github.basshift.Recall")
        .developer_name("Sebastian Dávila (Basshift)")
        .developers(vec!["Sebastian Dávila (Basshift)"])
        .version("1.0.0")
        .comments(tr("A memory game for finding pairs."))
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

pub fn create_keyboard_shortcuts_overlay() -> gtk::ShortcutsWindow {
    let game_title = xml_escape(&tr("Game"));
    let general_title = xml_escape(&tr("General"));
    let move_cards = xml_escape(&tr("Move between cards"));
    let flip_card = xml_escape(&tr("Flip selected card"));
    let game_action = xml_escape(&tr("Restart game or end run"));
    let back_to_menu = xml_escape(&tr("Back to menu"));
    let show_shortcuts = xml_escape(&tr("Show shortcuts"));
    let how_to_play = xml_escape(&tr("How to play"));
    let preferences = xml_escape(&tr("Preferences"));
    let back_main = xml_escape(&tr("Back to main menu"));
    let quit = xml_escape(&tr("Quit"));

    let xml = format!(
        r#"
<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkShortcutsWindow" id="help_overlay">
    <property name="modal">True</property>
    <property name="hide-on-close">True</property>
    <child>
      <object class="GtkShortcutsSection">
        <property name="section-name">shortcuts</property>
        <property name="max-height">12</property>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">{game_title}</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{move_cards}</property>
                <property name="accelerator">Up Down Left Right</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{flip_card}</property>
                <property name="accelerator">space Return</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{game_action}</property>
                <property name="accelerator">&lt;Primary&gt;r</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{back_to_menu}</property>
                <property name="accelerator">Escape</property>
              </object>
            </child>
          </object>
        </child>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">{general_title}</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{show_shortcuts}</property>
                <property name="accelerator">&lt;Primary&gt;slash</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{how_to_play}</property>
                <property name="accelerator">F1</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{preferences}</property>
                <property name="accelerator">&lt;Primary&gt;comma</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{back_main}</property>
                <property name="accelerator">&lt;Primary&gt;m</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">{quit}</property>
                <property name="accelerator">&lt;Primary&gt;q</property>
              </object>
            </child>
          </object>
        </child>
      </object>
    </child>
  </object>
</interface>
"#,
    );

    let builder = gtk::Builder::from_string(&xml);
    builder
        .object::<gtk::ShortcutsWindow>("help_overlay")
        .expect("failed to build keyboard shortcuts overlay")
}
