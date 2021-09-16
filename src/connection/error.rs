use std::io;
use thiserror::Error;

use super::Response;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Server speaks multiplex protocol other than protocol 4.")]
    UnsupportedMuxProtocol,

    #[error("Server response with unexpected package type {0}: Response {1:#?}.")]
    InvalidServerResponse(&'static str, Response),

    #[error("Server response with port = 0.")]
    InvalidPort,

    #[error("Server response with pid = 0.")]
    InvalidPid,

    #[error("Server response with a different id than the requested one.")]
    UnmatchedRequestId,

    #[error("Server response with a different session_id.")]
    UnmatchedSessionId,

    #[error("IO Error (Excluding `EWOULDBLOCK`): {0}.")]
    IOError(#[from] io::Error),

    #[error("Failed to serialize/deserialize the message: {0}.")]
    FormatError(#[from] ssh_mux_format::Error),

    #[error("Server refused the request: {0}.")]
    RequestFailure(String),

    #[error("Server refused the request due to insufficient permission: {0}.")]
    PermissionDenied(String),
}
