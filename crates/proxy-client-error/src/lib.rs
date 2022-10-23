pub use ssh_format_error::Error as SshFormatError;

mod error;
pub use error::Error;

mod open_failure;
pub use open_failure::{ErrMsg, ErrorCode, OpenFailure};
