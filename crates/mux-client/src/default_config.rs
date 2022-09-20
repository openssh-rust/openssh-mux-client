use std::{env, os::unix::ffi::OsStringExt};

use once_cell::sync::OnceCell;

use crate::{NonZeroByteSlice, NonZeroByteVec};

/// Return environment variable `$TERM` if set.
/// Otherwise, returns empty string.
pub fn get_term() -> &'static NonZeroByteSlice {
    static TERM: OnceCell<Option<NonZeroByteVec>> = OnceCell::new();
    TERM.get_or_init(|| {
        env::var_os("TERM")
            .map(OsStringExt::into_vec)
            .map(NonZeroByteVec::from_bytes_remove_nul)
    })
    .as_deref()
    .unwrap_or_else(|| NonZeroByteSlice::new(&[]).unwrap())
}
