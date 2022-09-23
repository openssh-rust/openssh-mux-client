use std::io;
use thiserror::Error;

pub use crate::response::error::*;

#[derive(Debug, Error)]
pub enum Error {
    /// Error when waiting for response
    #[error("Error when waiting for response: {0}.")]
    AwaitableError(#[from] awaitable::Error),

    /// IO Error (Excluding `io::ErrorKind::EWOULDBLOCK`).
    #[error("IO Error: {0}.")]
    IOError(#[from] io::Error),

    /// Failed to serialize/deserialize the message: {0}.
    #[error("Failed to serialize/deserialize the message: {0}.")]
    FormatError(#[from] ssh_format::Error),

    /// The response id is invalid.
    #[error("The response id {response_id} is invalid.")]
    InvalidResponseId {
        /// The invalid response id
        response_id: u32,
    },

    /// Invalid response from the sftp-server
    #[error("Response from sftp server is invalid: {0}")]
    InvalidResponse(
        // Use `&&str` since `&str` takes 16 bytes while `&str` only takes 8 bytes.
        &'static &'static str,
    ),
}
