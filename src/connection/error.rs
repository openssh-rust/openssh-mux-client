use std::convert::From;
use std::io;
use std::fmt;

use super::constants;

#[derive(Debug)]
pub enum Error {
    UnsupportedMuxProtocol,
    InvalidServerResponse(&'static str),
    UnmatchedRequestId,
    UnmatchedSessionId,
    IOError(io::Error),
    FormatError(ssh_mux_format::Error),
    RequestFailure(String),
    PermissionDenied(String),
    InvalidPort,
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
            UnsupportedMuxProtocol =>
                formatter.write_fmt(format_args!(
                    "Unsupported server protocol: {:#?}",
                    constants::UNSUPPORTED_MUX_PROTOCOL_ERRMSG
                )),
            InvalidServerResponse(msg) =>
                formatter.write_fmt(format_args!("Invalid server response: {:#?}", msg)),
            InvalidPort =>
                formatter.write_str("Invalid port from the server: Port must not be 0"),
            IOError(err) =>
                formatter.write_fmt(format_args!("IO Error: {:#?}", err)),
            FormatError(err) =>
                formatter.write_fmt(format_args!(
                    "Error in (de)serialization: {:#?}",
                    err
                )),
            UnmatchedRequestId =>
                formatter.write_str(
                    "The request_id server response with doesn't match with the request"
                ),
            UnmatchedSessionId => 
                formatter.write_str(
                    "The session_id server response with doesn't match with the request"
                ),
            RequestFailure(reason) =>
                formatter.write_fmt(format_args!("Request failed: {}", reason)),
            PermissionDenied(reason) =>
                formatter.write_fmt(format_args!("Permission denied: {}", reason)),
        }
    }
}

impl std::error::Error for Error {}
