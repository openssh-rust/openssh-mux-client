#[cfg(not(unix))]
compile_error!("This crate can only be used on unix");

mod connection;
mod constants;
mod error;
mod request;
mod response;
mod session;
mod shutdown_mux_master;
mod utils;

pub mod default_config;

pub use non_zero_byte_slice::*;

pub use error::Error;
pub type Result<T, Err = Error> = std::result::Result<T, Err>;

pub use request::{Session, Socket};
pub use response::Response;

pub use session::*;

pub use connection::*;

pub use shutdown_mux_master::shutdown_mux_master;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
