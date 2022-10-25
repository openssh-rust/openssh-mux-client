use std::io;
use thiserror::Error as ThisError;

use crate::{OpenFailure, SshFormatError};

#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum Error {
    /// IO Error (Excluding `io::ErrorKind::EWOULDBLOCK`).
    #[error("IO Error: {0}.")]
    IOError(#[from] io::Error),

    /// Failed to serialize/deserialize the message: {0}.
    #[error("Failed to serialize/deserialize the message: {0}.")]
    FormatError(#[from] SshFormatError),

    /// Invalid response from the sshd
    #[error("Response from sshd is invalid: {0}")]
    InvalidResponse(
        /// Use `&&str` since `&str` takes 16 bytes while `&str` only takes 8 bytes.
        &'static &'static str,
    ),

    /// Failed to open channel
    #[error(transparent)]
    ChannelOpenFailure(#[from] OpenFailure),

    /// Unexpected channel state
    #[error("Expected {expected_state} but actual state is {actual_state}")]
    UnexpectedChannelState {
        expected_state: &'static &'static str,
        actual_state: &'static str,
    },

    /// Invalid recipient channel id
    #[error("Invalid recipient channel id {0}")]
    InvalidRecipientChannel(u32),
}

impl From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        match err {
            Error::IOError(io_error) => io_error,
            other => io::Error::new(io::ErrorKind::Other, other),
        }
    }
}
