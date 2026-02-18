use std::rc::Rc;
use std::cell::RefCell;
use gtk4 as gtk;
use gtk4::prelude::*;
use gtk4::pango;
use super::state::{AppState, TileStatus};
use super::app::handle_tile_click;

pub const CONTENT_MARGIN: i32 = 12;
pub const TILE_GAP: i32 = 6;

pub fn build_board_grid(state: &Rc<RefCell<AppState>>) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.add_css_class("recall-board");
    grid.set_row_spacing(TILE_GAP as u32);
    grid.set_column_spacing(TILE_GAP as u32);
    grid.set_halign(gtk::Align::Fill);
    grid.set_valign(gtk::Align::Fill);
    grid.set_hexpand(true);
    grid.set_vexpand(true);

    let css_provider = {
        let st = state.borrow();
        st.dynamic_css_provider.clone()
    };

    let (grid_cols, grid_rows) = {
        let st = state.borrow();
        (st.grid_cols, st.grid_rows)
    };

    let update_styles = {
        let css_provider = css_provider.clone();
        move |grid: &gtk::Grid| {
            let width = grid.width();
            let height = grid.height();
            if width > 0 && height > 0 {
                let cell_width = (width - (grid_cols - 1) * TILE_GAP) / grid_cols;
                let cell_height = (height - (grid_rows - 1) * TILE_GAP) / grid_rows;
                let min_dim = cell_width.min(cell_height);
                
                // Dynamic radii based on available cell size.
                let card_radius = (min_dim as f64 * 0.15) as i32;
                let container_radius = (min_dim as f64 * 0.25) as i32;

                if let Some(provider) = &css_provider {
                    provider.load_from_data(&format!(
                        ".recall-card {{ border-radius: {card_radius}px; }} \
                         .recall-card-container {{ border-radius: {container_radius}px; }}",
                        card_radius = card_radius,
                        container_radius = container_radius
                    ));
                }
            }
        }
    };

    let update_styles_clone = update_styles.clone();
    grid.connect_closure(
        "notify::width",
        false,
        glib::closure_local!(move |grid: gtk::Grid, _: glib::ParamSpec| {
            update_styles_clone(&grid);
        }),
    );
    grid.connect_closure(
        "notify::height",
        false,
        glib::closure_local!(move |grid: gtk::Grid, _: glib::ParamSpec| {
            update_styles(&grid);
        }),
    );

    let mut buttons = Vec::new();

    for i in 0..(grid_rows * grid_cols) {
        let index = i as usize;
        let aspect_frame = gtk::AspectFrame::builder()
            .ratio(1.0)
            .obey_child(false)
            .halign(gtk::Align::Fill)
            .valign(gtk::Align::Fill)
            .hexpand(true)
            .vexpand(true)
            .build();

        let button = gtk::Button::builder()
            .css_classes(vec!["recall-card"])
            .build();
        button.set_hexpand(true);
        button.set_vexpand(true);
        
        let drawing_area = gtk::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        drawing_area.add_css_class("recall-card-label");

        let state_draw = state.clone();
        drawing_area.set_draw_func(move |area, cr, width, height| {
            let st = state_draw.borrow();
            if index >= st.tiles.len() {
                return;
            }
            let tile = &st.tiles[index];
            let is_hidden = tile.status == TileStatus::Hidden;
            let text = if !is_hidden { &tile.value } else { "?" };

            let min_dim = width.min(height) as f64;
            let font_size = if is_hidden {
                min_dim * 0.34
            } else {
                min_dim * 0.40
            };

            cr.set_antialias(gtk::cairo::Antialias::Best);

            let layout = pangocairo::functions::create_layout(cr);
            let mut font_desc = pango::FontDescription::new();
            if is_hidden {
                font_desc.set_family("Cantarell, Noto Sans, sans");
                font_desc.set_weight(pango::Weight::Bold);
            } else {
                font_desc.set_family("Noto Color Emoji, Apple Color Emoji, Segoe UI Emoji, sans");
            }
            font_desc.set_size((font_size * pango::SCALE as f64) as i32);
            layout.set_font_description(Some(&font_desc));
            layout.set_text(text);

            let fg = area.style_context().color();
            cr.set_source_rgba(
                fg.red() as f64,
                fg.green() as f64,
                fg.blue() as f64,
                fg.alpha() as f64,
            );

            let (text_width, text_height) = layout.pixel_size();
            cr.move_to(
                (width as f64 - text_width as f64) / 2.0,
                (height as f64 - text_height as f64) / 2.0,
            );

            pangocairo::functions::show_layout(cr, &layout);
        });

        button.set_child(Some(&drawing_area));

        if let Some(tile) = state.borrow().tiles.get(index) {
            match tile.status {
                TileStatus::Matched => button.add_css_class("matched"),
                TileStatus::Flipped => button.add_css_class("active"),
                TileStatus::Hidden => (),
            }
        }

        let state_clone = state.clone();
        button.connect_clicked(move |_| {
            handle_tile_click(&state_clone, i as usize);
        });

        aspect_frame.set_child(Some(&button));

        let x = i % grid_cols;
        let y = i / grid_cols;
        grid.attach(&aspect_frame, x, y, 1, 1);
        buttons.push(button);
    }

    state.borrow_mut().grid_buttons = buttons;

    grid
}
