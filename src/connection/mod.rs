mod constants;
mod error;
mod request;
mod response;
mod raw_connection;

use request::Request;
use response::Response;
use raw_connection::RawConnection;

use core::num::Wrapping;
use core::convert::AsRef;
use core::mem;
use std::path::Path;

use serde::{Serialize, Deserialize};
use ssh_mux_format::{Serializer, from_bytes};

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub struct Connection {
    raw_conn: RawConnection,
    serializer: Serializer,
    /// Buffer for input from the server
    buffer: Vec<u8>,
    request_id: Wrapping<u32>,
}
impl Connection {
    async fn write(&mut self, value: &Request<'_>) -> Result<()> {
        value.serialize(&mut self.serializer)?;

        self.raw_conn.write(self.serializer.get_output()?).await?;
        self.serializer.reset();

        Ok(())
    }

    async fn read_and_deserialize<'a, T>(&'a mut self, size: usize) -> Result<T>
    where
        T: Deserialize<'a>
    {
        self.buffer.resize(size, 0);
        self.raw_conn.read(&mut self.buffer).await?;
        Ok(from_bytes(&self.buffer)?)
    }

    /// Return size of the response.
    async fn read_header(&mut self) -> Result<u32> {
        self.read_and_deserialize(4).await
    }

    async fn read_response(&mut self) -> Result<Response> {
        let len = self.read_header().await?;
        self.read_and_deserialize(len as usize).await
    }

    fn get_request_id(&mut self) -> u32 {
        let request_id = self.request_id.0;
        self.request_id += Wrapping(1);

        request_id
    }

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut conn = Self {
            raw_conn: RawConnection::connect(path).await?,
            serializer: Serializer::new(),
            buffer: Vec::with_capacity(mem::size_of::<Response>()),
            request_id: Wrapping(0),
        };

        conn.write(&Request::Hello { version: constants::SSHMUX_VER }).await?;

        let response = conn.read_response().await?;
        if let Response::Hello { version } = response {
            if version != constants::SSHMUX_VER {
                Err(Error::UnsupportedMuxProtocol)
            } else {
                Ok(conn)
            }
        } else {
            Err(Error::InvalidServerResponse("Expected Hello message"))
        }
    }

    /// Return pid of the ssh mux server.
    pub async fn send_alive_check(&mut self) -> Result<u32> {
        let request_id = self.get_request_id();

        self.write(&Request::AliveCheck { request_id }).await?;

        let response = self.read_response().await?;
        if let Response::Alive { request_id: server_request_id, server_pid } = response {
            if request_id != server_request_id {
                Err(Error::UnmatchedRequestId)
            } else {
                Ok(server_pid)
            }
        } else {
            Err(Error::InvalidServerResponse("Expected Response::Alive"))
        }
    }
}
