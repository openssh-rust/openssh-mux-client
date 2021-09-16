use once_cell::sync::OnceCell;
use std::env;

pub fn get_term() -> &'static str {
    static TERM: OnceCell<String> = OnceCell::new();
    TERM.get_or_init(|| env::var("TERM").unwrap_or_default())
}
