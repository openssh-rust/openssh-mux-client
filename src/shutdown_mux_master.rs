#![forbid(unsafe_code)]

use crate::{constants, request::Request, Error, Response, Result};

use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;

use serde::Deserialize;
use ssh_format::Transformer;

struct Connection {
    raw_conn: UnixStream,
    transformer: Transformer,
}

impl Connection {
    fn write(&mut self, value: &Request<'_>) -> Result<()> {
        self.raw_conn
            .write_all(self.transformer.serialize(value)?)?;

        Ok(())
    }

    fn read_and_deserialize<'a, T>(&'a mut self, size: usize) -> Result<T>
    where
        T: Deserialize<'a>,
    {
        self.transformer.get_buffer().resize(size, 0);
        self.raw_conn.read_exact(self.transformer.get_buffer())?;

        // Ignore any trailing bytes to be forward compatible
        Ok(self.transformer.deserialize()?.0)
    }

    /// Return size of the response.
    fn read_header(&mut self) -> Result<u32> {
        self.read_and_deserialize(4)
    }

    fn read_response(&mut self) -> Result<Response> {
        let len = self.read_header()?;
        self.read_and_deserialize(len as usize)
    }

    fn check_response_id(request_id: u32, response_id: u32) -> Result<()> {
        if request_id != response_id {
            Err(Error::UnmatchedRequestId)
        } else {
            Ok(())
        }
    }

    fn exchange_hello(mut self) -> Result<Self> {
        self.write(&Request::Hello {
            version: constants::SSHMUX_VER,
        })?;

        let response = self.read_response()?;
        if let Response::Hello { version } = response {
            if version != constants::SSHMUX_VER {
                Err(Error::UnsupportedMuxProtocol)
            } else {
                Ok(self)
            }
        } else {
            Err(Error::InvalidServerResponse(
                "Expected Hello message",
                response,
            ))
        }
    }

    fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self {
            raw_conn: UnixStream::connect(path)?,
            transformer: Transformer::new(),
        }
        .exchange_hello()
    }

    /// Request the master to stop accepting new multiplexing requests
    /// and remove its listener socket.
    fn request_stop_listening(&mut self) -> Result<()> {
        use Response::*;

        let request_id = 0;
        self.write(&Request::StopListening { request_id })?;

        match self.read_response()? {
            Ok { response_id } => {
                Self::check_response_id(request_id, response_id)?;
                Result::Ok(())
            }
            PermissionDenied {
                response_id,
                reason,
            } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::PermissionDenied(reason))
            }
            Failure {
                response_id,
                reason,
            } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::RequestFailure(reason))
            }
            response => Err(Error::InvalidServerResponse(
                "Expected Response: Ok, PermissionDenied or Failure",
                response,
            )),
        }
    }
}

/// Request the master to stop accepting new multiplexing requests
/// and remove its listener socket.
///
/// **Only suitable to use in `Drop::drop`.**
pub fn shutdown_mux_master<P: AsRef<Path>>(path: P) -> Result<()> {
    Connection::connect(path)?.request_stop_listening()
}

pub(crate) fn shutdown_mux_master_from(raw_conn: UnixStream) -> Result<()> {
    Connection {
        raw_conn,
        transformer: Transformer::new(),
    }
    .request_stop_listening()
}

#[cfg(test)]
mod tests {
    use super::shutdown_mux_master;

    #[test]
    fn test_sync_request_stop_listening() {
        shutdown_mux_master("/tmp/openssh-mux-client-test.socket").unwrap();
    }
}
