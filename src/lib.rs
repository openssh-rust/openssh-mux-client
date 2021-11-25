#![forbid(unsafe_code)]

mod connection;
mod constants;
mod error;
mod raw_connection;
mod request;
mod response;
mod session;

pub mod default_config;

pub use error::Error;
pub type Result<T, Err = Error> = std::result::Result<T, Err>;

pub use request::{Session, Socket};
pub use response::Response;

pub use session::*;

pub use connection::*;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
