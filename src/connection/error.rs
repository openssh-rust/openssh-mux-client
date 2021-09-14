use std::convert::From;
use std::io;
use std::fmt;

use super::constants;

#[derive(Debug)]
pub enum Error {
    UnsupportedMuxProtocol,
    InvalidServerResponse(&'static str),
    UnmatchedRequestId,
    IOError(io::Error),
    FormatError(ssh_mux_format::Error),
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
            IOError(err) =>
                formatter.write_fmt(format_args!("IO Error: {:#?}", err)),
            FormatError(err) =>
                formatter.write_fmt(format_args!(
                    "Error in (de)serialization: {:#?}",
                    err
                )),
            UnmatchedRequestId =>
                formatter.write_str(
                    "The request_id server response with doesn't match the request"
                ),
        }
    }
}

impl std::error::Error for Error {}
