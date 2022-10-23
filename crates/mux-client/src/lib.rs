#[cfg(not(unix))]
compile_error!("This crate can only be used on unix");

pub use non_zero_byte_slice::*;

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

pub mod default_config;

mod connection;
pub use connection::*;

mod constants;

mod request;
pub use request::{Session, Socket};

mod response;
pub use response::Response;

mod session;
pub use session::*;

mod shutdown_mux_master;
pub use shutdown_mux_master::shutdown_mux_master;

mod utils;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
