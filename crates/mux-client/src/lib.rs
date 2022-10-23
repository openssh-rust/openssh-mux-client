#[cfg(not(unix))]
compile_error!("This crate can only be used on unix");

pub use error::Error;
pub use openssh_mux_client_error as error;
pub type Result<T, Err = Error> = std::result::Result<T, Err>;

trait ErrorExt {
    fn invalid_server_response(package_type: &'static &'static str, response: &Response) -> Self;
}

impl ErrorExt for Error {
    fn invalid_server_response(package_type: &'static &'static str, response: &Response) -> Self {
        Error::InvalidServerResponse(package_type, format!("{:#?}", response).into_boxed_str())
    }
}

mod connection;
mod constants;
mod request;
mod response;
mod session;
mod shutdown_mux_master;
mod utils;

pub mod default_config;

pub use non_zero_byte_slice::*;

pub use request::{Session, Socket};
pub use response::Response;

pub use session::*;

pub use connection::*;

pub use shutdown_mux_master::shutdown_mux_master;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
