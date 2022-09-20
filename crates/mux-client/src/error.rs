use std::io;
use thiserror::Error;

use super::Response;

#[derive(Debug, Error)]
pub enum Error {
    /// Server speaks multiplex protocol other than protocol 4.
    #[error("Server speaks multiplex protocol other than protocol 4.")]
    UnsupportedMuxProtocol,

    /// Server response with unexpected package type {0}: Response {1:#?}.
    #[error("Server response with unexpected package type {0}: Response {1:#?}.")]
    InvalidServerResponse(&'static str, Response),

    /// Server response with port = 0.
    #[error("Server response with port = 0.")]
    InvalidPort,

    /// Server response with pid = 0.
    #[error("Server response with pid = 0.")]
    InvalidPid,

    /// Server response with a different id than the requested one.
    #[error("Server response with a different id than the requested one.")]
    UnmatchedRequestId,

    /// Server response with a different session_id.
    #[error("Server response with a different session_id.")]
    UnmatchedSessionId,

    /// IO Error (Excluding `EWOULDBLOCK`): {0}.
    #[error("IO Error (Excluding `EWOULDBLOCK`): {0}.")]
    IOError(#[from] io::Error),

    /// Failed to serialize/deserialize the message: {0}.
    #[error("Failed to serialize/deserialize the message: {0}.")]
    FormatError(#[from] ssh_format::Error),

    /// Server refused the request: {0}.
    #[error("Server refused the request: {0}.")]
    RequestFailure(Box<str>),

    /// Server refused the request due to insufficient permission: {0}.
    #[error("Server refused the request due to insufficient permission: {0}.")]
    PermissionDenied(Box<str>),
}
