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

pub use std::os::unix::io::RawFd;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub use request::Session;

#[derive(Debug)]
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

    fn check_response_id(request_id: u32, response_id: u32) -> Result<()> {
        if request_id != response_id {
            Err(Error::UnmatchedRequestId)
        } else {
            Ok(())
        }
    }

    async fn exchange_hello(mut self) -> Result<Self> {
        self.write(&Request::Hello { version: constants::SSHMUX_VER }).await?;

        let response = self.read_response().await?;
        if let Response::Hello { version } = response {
            if version != constants::SSHMUX_VER {
                Err(Error::UnsupportedMuxProtocol)
            } else {
                Ok(self)
            }
        } else {
            Err(Error::InvalidServerResponse("Expected Hello message"))
        }
    }

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self {
            raw_conn: RawConnection::connect(path).await?,
            serializer: Serializer::new(),
            buffer: Vec::with_capacity(mem::size_of::<Response>()),
            request_id: Wrapping(0),
        }.exchange_hello().await
    }

    /// Return pid of the ssh mux server.
    pub async fn send_alive_check(&mut self) -> Result<u32> {
        let request_id = self.get_request_id();

        self.write(&Request::AliveCheck { request_id }).await?;

        let response = self.read_response().await?;
        if let Response::Alive { response_id, server_pid } = response {
            Self::check_response_id(request_id, response_id)?;
            Ok(server_pid)
        } else {
            Err(Error::InvalidServerResponse("Expected Response::Alive"))
        }
    }

    pub async fn open_new_session(mut self, session: &Session<'_>, fds: &[RawFd; 3])
        -> Result<EstablishedSession>
    {
        use Response::*;

        let request_id = self.get_request_id();

        let reserved = "";
        self.write(&Request::NewSession { request_id, reserved, session }).await?;

        let session_id = match self.read_response().await? {
            SessionOpened { response_id, session_id } => {
                Self::check_response_id(request_id, response_id)?;
                session_id
            },
            PermissionDenied { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                return Err(Error::PermissionDenied(reason))
            },
            Failure { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                return Err(Error::RequestFailure(reason))
            },
            _ => return Err(Error::InvalidServerResponse(
                "Expected Response: SessionOpened, PermissionDenied or Failure"
            )),
        };

        self.raw_conn.send_fds(&fds[..])?;

        if let Ok { response_id } = self.read_response().await? {
            Self::check_response_id(request_id, response_id)?;
            Result::Ok(EstablishedSession {
                conn: self,
                session_id,
            })
        } else {
            Err(Error::InvalidServerResponse("Expected Response::Ok"))
        }
    }
}

pub struct EstablishedSession {
    conn: Connection,
    session_id: u32,
}
impl EstablishedSession {
    fn check_session_id(&self, session_id: u32) -> Result<()> {
        if self.session_id != session_id {
            Err(Error::UnmatchedSessionId)
        } else {
            Ok(())
        }
    }

    /// Wait for session status to change
    pub async fn wait(mut self) -> Result<SessionStatus> {
        use Response::*;

        match self.conn.read_response().await? {
            TtyAllocFail { session_id } => {
                self.check_session_id(session_id)?;
                Result::Ok(SessionStatus::TtyAllocFail(self))
            },
            ExitMessage { session_id, exit_value } => {
                self.check_session_id(session_id)?;
                Result::Ok(SessionStatus::Exited {
                    conn: self.conn,
                    exit_value,
                })
            },
            _ => Err(Error::InvalidServerResponse(
                "Expected Response TtyAllocFail or ExitMessage"
            ))
        }
    }
}

pub enum SessionStatus {
    TtyAllocFail(EstablishedSession),
    Exited {
        conn: Connection,
        exit_value: u32,
    },
}
