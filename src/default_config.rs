use once_cell::sync::OnceCell;
use std::env;

/// Return environment variable `$TERM` if set.
/// Otherwise, returns empty string.
pub fn get_term() -> &'static str {
    static TERM: OnceCell<String> = OnceCell::new();
    TERM.get_or_init(|| env::var("TERM").unwrap_or_default())
}
