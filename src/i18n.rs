use std::path::PathBuf;

use gettextrs::{
    LocaleCategory, bind_textdomain_codeset, bindtextdomain, dgettext, setlocale, textdomain,
};

pub const GETTEXT_PACKAGE: &str = "io.github.basshift.Recall";

pub fn init() {
    let locale_dir = std::env::var("RECALL_LOCALEDIR").unwrap_or_else(|_| default_locale_dir());

    setlocale(LocaleCategory::LcAll, "");
    if bindtextdomain(GETTEXT_PACKAGE, locale_dir).is_err() {
        eprintln!("warning: failed to bind gettext domain to locale dir");
    }
    if bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").is_err() {
        eprintln!("warning: failed to bind gettext domain to UTF-8 codeset");
    }
    if textdomain(GETTEXT_PACKAGE).is_err() {
        eprintln!("warning: failed to activate gettext domain");
    }
}

fn default_locale_dir() -> String {
    let flatpak_locale_dir = "/app/share/locale";
    if std::path::Path::new(flatpak_locale_dir).exists() {
        return flatpak_locale_dir.to_string();
    }

    let project_locale_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("po");
    project_locale_dir.to_string_lossy().into_owned()
}

pub fn tr(message: &str) -> String {
    if message.contains('\0') {
        eprintln!("warning: gettext key contains interior null byte");
        return message.to_string();
    }

    dgettext(GETTEXT_PACKAGE, message)
}
