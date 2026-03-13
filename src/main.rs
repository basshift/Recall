mod i18n;
mod ui;

fn main() {
    i18n::init();
    ui::app::run();
}
