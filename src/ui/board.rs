use std::cell::{Cell, RefCell};
use std::rc::Rc;
use gtk4 as gtk;
use gtk4::prelude::*;
use gtk4::pango;
use super::state::{AppState, TileStatus};
use super::app::handle_tile_click;

pub const CONTENT_MARGIN: i32 = 12;
pub const TILE_GAP: i32 = 6;
const CARD_RADIUS_FACTOR: f64 = 0.12;
const CONTAINER_RADIUS_FACTOR: f64 = 0.20;
const CARD_RADIUS_MIN: i32 = 4;
const CARD_RADIUS_MAX: i32 = 14;
const CONTAINER_RADIUS_MIN: i32 = 8;
const CONTAINER_RADIUS_MAX: i32 = 24;
const TILE_GAP_MIN: i32 = 2;
const CONTAINER_PADDING_FACTOR: f64 = 0.20;
const CONTAINER_PADDING_MIN: i32 = 6;
const CONTAINER_PADDING_MAX: i32 = 24;

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

    let update_styles = {
        let state = state.clone();
        let css_provider = css_provider.clone();
        move |grid: &gtk::Grid| {
            let width = grid.allocated_width();
            let height = grid.allocated_height();
            if width > 0 && height > 0 {
                let (grid_cols, grid_rows) = {
                    let st = state.borrow();
                    (st.grid_cols.max(1), st.grid_rows.max(1))
                };
                let grid_cells = grid_cols.max(grid_rows).max(1);
                let approx_cell = width.min(height) / grid_cells;
                let tile_gap =
                    ((approx_cell as f64 * 0.10).round() as i32).clamp(TILE_GAP_MIN, TILE_GAP);
                grid.set_row_spacing(tile_gap as u32);
                grid.set_column_spacing(tile_gap as u32);

                let cell_width = (width - (grid_cols - 1) * tile_gap) / grid_cols;
                let cell_height = (height - (grid_rows - 1) * tile_gap) / grid_rows;
                let min_dim = cell_width.min(cell_height);
                
                // Dynamic radii based on available cell size.
                let card_radius = ((min_dim as f64 * CARD_RADIUS_FACTOR).round() as i32)
                    .clamp(CARD_RADIUS_MIN, CARD_RADIUS_MAX);
                let container_radius =
                    ((min_dim as f64 * CONTAINER_RADIUS_FACTOR).round() as i32)
                        .clamp(CONTAINER_RADIUS_MIN, CONTAINER_RADIUS_MAX);
                let container_padding =
                    ((min_dim as f64 * CONTAINER_PADDING_FACTOR).round() as i32)
                        .clamp(CONTAINER_PADDING_MIN, CONTAINER_PADDING_MAX);

                if let Some(provider) = &css_provider {
                    provider.load_from_data(&format!(
                        "window.app-window .recall-card {{ border-radius: {card_radius}px; }} \
                         window.app-window .recall-card-container {{ border-radius: {container_radius}px; padding: {container_padding}px; }}",
                        card_radius = card_radius,
                        container_radius = container_radius,
                        container_padding = container_padding
                    ));
                }
            }
        }
    };

    let last_size = Rc::new(Cell::new((0, 0)));
    let update_styles_tick = update_styles.clone();
    let last_size_tick = last_size.clone();
    grid.add_tick_callback(move |grid, _| {
        let size = (grid.allocated_width(), grid.allocated_height());
        if size.0 > 0 && size.1 > 0 && size != last_size_tick.get() {
            last_size_tick.set(size);
            update_styles_tick(grid);
        }
        glib::ControlFlow::Continue
    });

    let mut buttons = Vec::new();

    let (grid_cols, grid_rows) = {
        let st = state.borrow();
        (st.grid_cols, st.grid_rows)
    };

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

            cr.set_antialias(gtk::cairo::Antialias::Default);

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
            let text_x = (width - text_width) as f64 / 2.0;
            let text_y = (height - text_height) as f64 / 2.0;
            cr.move_to(text_x, text_y);

            pangocairo::functions::show_layout(cr, &layout);
        });

        button.set_child(Some(&drawing_area));

        if let Some(tile) = state.borrow().tiles.get(index) {
            match tile.status {
                TileStatus::Matched => {
                    button.add_css_class("matched");
                    button.add_css_class("matched-dim");
                }
                TileStatus::Flipped => button.add_css_class("active"),
                TileStatus::Hidden => (),
            }
        }

        let state_clone = state.clone();
        button.connect_clicked(move |_| {
            handle_tile_click(&state_clone, i as usize);
        });
        let state_mouse_enter = state.clone();
        let motion = gtk::EventControllerMotion::new();
        motion.connect_enter(move |_, _, _| {
            let st = state_mouse_enter.borrow();
            for button in &st.grid_buttons {
                button.remove_css_class("kbd-focus");
            }
        });
        button.add_controller(motion);

        aspect_frame.set_child(Some(&button));

        let x = i % grid_cols;
        let y = i / grid_cols;
        grid.attach(&aspect_frame, x, y, 1, 1);
        buttons.push(button);
    }

    state.borrow_mut().grid_buttons = buttons;

    grid
}
