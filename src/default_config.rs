use std::env;
use std::ffi::{CStr, CString};

use once_cell::sync::OnceCell;

/// Return environment variable `$TERM` if set.
/// Otherwise, returns empty string.
pub fn get_term() -> &'static CStr {
    static TERM: OnceCell<CString> = OnceCell::new();
    TERM.get_or_init(|| CString::new(env::var("TERM").unwrap_or_default()).unwrap())
}
