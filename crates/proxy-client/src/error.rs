use std::io;
use thiserror::Error;

pub use crate::response::error::*;

#[derive(Debug, Error)]
pub enum Error {
    /// IO Error (Excluding `io::ErrorKind::EWOULDBLOCK`).
    #[error("IO Error: {0}.")]
    IOError(#[from] io::Error),

    /// Failed to serialize/deserialize the message: {0}.
    #[error("Failed to serialize/deserialize the message: {0}.")]
    FormatError(#[from] ssh_format::Error),

    /// Invalid response from the sshd
    #[error("Response from sshd is invalid: {0}")]
    InvalidResponse(
        /// Use `&&str` since `&str` takes 16 bytes while `&str` only takes 8 bytes.
        &'static &'static str,
    ),

    /// Failed to open channel
    #[error(transparent)]
    ChannelOpenFailure(#[from] OpenFailure),
}
