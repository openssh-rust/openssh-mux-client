use std::convert::From;
use std::fmt;
use std::io;

use super::{constants, Response};

#[derive(Debug)]
pub enum Error {
    /// Server speaks a different multiple protocol.
    UnsupportedMuxProtocol,

    /// Server response with unexpected package type.
    InvalidServerResponse(&'static str, Response),

    /// Server response with port = 0.
    InvalidPort,

    /// Server response with pid = 0.
    InvalidPid,

    /// Server response with a different id than the requested one.
    UnmatchedRequestId,

    /// Server response with a different session_id.
    UnmatchedSessionId,

    /// IO Error (Excluding `EWOULDBLOCK`).
    IOError(io::Error),

    /// Failed to serialize/deserialize the message using crate `ssh_mux_format`.
    FormatError(ssh_mux_format::Error),

    /// Server refused the request with a reason.
    RequestFailure(String),

    /// Server refused the request due to insufficient permission with a reason.
    PermissionDenied(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IOError(err)
    }
}
impl From<ssh_mux_format::Error> for Error {
    fn from(err: ssh_mux_format::Error) -> Self {
        Error::FormatError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            UnsupportedMuxProtocol => formatter.write_fmt(format_args!(
                "Unsupported server protocol: {:#?}",
                constants::UNSUPPORTED_MUX_PROTOCOL_ERRMSG
            )),
            InvalidServerResponse(msg, response) => formatter.write_fmt(format_args!(
                "Invalid server response: {}, Actual response: {:#?}",
                msg, response
            )),
            InvalidPort => formatter.write_str("Invalid port from the server: Port must not be 0"),
            InvalidPid => formatter.write_str("Invalid pid from the server: Pid must not be 0"),
            IOError(err) => formatter.write_fmt(format_args!("IO Error: {:#?}", err)),
            FormatError(err) => {
                formatter.write_fmt(format_args!("Error in (de)serialization: {:#?}", err))
            }
            UnmatchedRequestId => formatter
                .write_str("The request_id server response with doesn't match with the request"),
            UnmatchedSessionId => formatter
                .write_str("The session_id server response with doesn't match with the request"),
            RequestFailure(reason) => {
                formatter.write_fmt(format_args!("Request failed: {}", reason))
            }
            PermissionDenied(reason) => {
                formatter.write_fmt(format_args!("Permission denied: {}", reason))
            }
        }
    }
}

impl std::error::Error for Error {}
