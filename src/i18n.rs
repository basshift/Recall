use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

pub const GETTEXT_PACKAGE: &str = "io.github.basshift.Recall";
const LC_ALL: c_int = 6;

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn bindtextdomain(domainname: *const c_char, dirname: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domainname: *const c_char, codeset: *const c_char) -> *mut c_char;
    fn textdomain(domainname: *const c_char) -> *mut c_char;
    fn dgettext(domainname: *const c_char, msgid: *const c_char) -> *mut c_char;
}

pub fn init() {
    let domain = CString::new(GETTEXT_PACKAGE).expect("invalid gettext domain");
    let locale_dir =
        std::env::var("RECALL_LOCALEDIR").unwrap_or_else(|_| "/app/share/locale".to_string());
    let locale_dir = CString::new(locale_dir).expect("invalid locale dir");
    let utf8 = CString::new("UTF-8").expect("invalid codeset");

    unsafe {
        setlocale(LC_ALL, c"".as_ptr());
        bindtextdomain(domain.as_ptr(), locale_dir.as_ptr());
        bind_textdomain_codeset(domain.as_ptr(), utf8.as_ptr());
        textdomain(domain.as_ptr());
    }
}

pub fn tr(message: &str) -> String {
    let domain = CString::new(GETTEXT_PACKAGE).expect("invalid gettext domain");
    let msg = CString::new(message).unwrap_or_else(|_| CString::new("").expect("empty cstring"));
    unsafe {
        let translated = dgettext(domain.as_ptr(), msg.as_ptr());
        if translated.is_null() {
            return message.to_string();
        }
        CStr::from_ptr(translated).to_string_lossy().into_owned()
    }
}
