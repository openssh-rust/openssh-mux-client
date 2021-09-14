mod constants;
mod error;
mod request;
mod response;
mod raw_connection;
mod session;

use request::Request;
use response::Response;
use raw_connection::RawConnection;
use request::Fwd;

use core::num::{Wrapping, NonZeroU32};
use core::convert::AsRef;
use core::mem;
use std::path::Path;

use serde::{Serialize, Deserialize};
use ssh_mux_format::{Serializer, from_bytes};

pub use std::os::unix::io::RawFd;

pub use error::Error;
pub type Result<T, Err = Error> = std::result::Result<T, Err>;

pub use request::{Session, Socket};
pub use session::*;

#[derive(Copy, Clone, Debug)]
pub enum ForwardType {
    Local,
    Remote,
}

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

    async fn open_new_session_impl(&mut self, session: &Session<'_>, fds: &[RawFd; 3])
        -> Result<(Response, u32, u32)>
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

        Result::Ok((self.read_response().await?, request_id, session_id))
    }

    /// Consumes `self` so that users would not be able to create multiple sessions
    /// or perform other operations during the session that might complicates the
    /// handling of packets received from the ssh mux server.
    ///
    /// The return value `EstablishedSession` will contain the moved `self`, which once
    /// the session has exited, you can get back this `Connection` and reused it.
    ///
    /// Return `Self` so that you can handle the error and reuse
    /// the `Connection`.
    pub async fn open_new_session(mut self, session: &Session<'_>, fds: &[RawFd; 3])
        -> Result<EstablishedSession, (Error, Self)>
    {
        let result = self.open_new_session_impl(session, fds).await;
        let (response, request_id, session_id) = match result {
            Result::Ok(result) => result,
            Err(err) => return Err((err, self)),
        };

        if let Response::Ok { response_id } = response {
            if let Err(err) = Self::check_response_id(request_id, response_id) {
                Err((err, self))
            } else {
                Ok(EstablishedSession {
                    conn: self,
                    session_id,
                })
            }
        } else {
            Err((Error::InvalidServerResponse("Expected Response::Ok"), self))
        }
    }

    pub async fn request_port_forward(
        &mut self,
        forward_type: ForwardType,
        listen_socket: &Socket<'_>,
        connect_socket: &Socket<'_>
    ) -> Result<()> {
        use ForwardType::*;
        use Response::*;

        let fwd = match forward_type {
            Local => {
                Fwd::Local {
                    listen_socket,
                    connect_socket,
                }
            },
            Remote => {
                Fwd::Remote {
                    listen_socket,
                    connect_socket,
                }
            },
        };
        let fwd = &fwd;

        let request_id = self.get_request_id();
        self.write(&Request::OpenFwd { request_id, fwd }).await?;

        match self.read_response().await? {
            Ok { response_id } => {
                Self::check_response_id(request_id, response_id)
            },
            PermissionDenied { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::PermissionDenied(reason))
            },
            Failure { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::RequestFailure(reason))
            },
            _ => Err(Error::InvalidServerResponse(
                "Expected Response: Ok, PermissionDenied or Failure"
            )),
        }
    }

    /// Return remote port opened for dynamic forwarding.
    pub async fn request_dynamic_forward(&mut self, listen_socket: &Socket<'_>)
        -> Result<NonZeroU32>
    {
        use Response::*;

        let fwd = Fwd::Dynamic { listen_socket };
        let fwd = &fwd;

        let request_id = self.get_request_id();
        self.write(&Request::OpenFwd { request_id, fwd }).await?;

        match self.read_response().await? {
            RemotePort { response_id, remote_port } => {
                Self::check_response_id(request_id, response_id)?;
                NonZeroU32::new(remote_port)
                    .ok_or(Error::InvalidPort)
            },
            PermissionDenied { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::PermissionDenied(reason))
            },
            Failure { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::RequestFailure(reason))
            },
            _ => Err(Error::InvalidServerResponse(
                "Expected Response: RemotePort, PermissionDenied or Failure"
            )),
        }
    }

    /// Request the master to stop accepting new multiplexing requests and remove its
    /// listener socket.
    pub async fn request_stop_listening(&mut self) -> Result<()> {
        use Response::*;

        let request_id = self.get_request_id();
        self.write(&Request::StopListening { request_id }).await?;

        match self.read_response().await? {
            Ok { response_id } => {
                Self::check_response_id(request_id, response_id)?;
                Result::Ok(())
            },
            PermissionDenied { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::PermissionDenied(reason))
            },
            Failure { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::RequestFailure(reason))
            },
            _ => Err(Error::InvalidServerResponse(
                "Expected Response: Ok, PermissionDenied or Failure"
            )),
        }
    }

    async fn request_terminate_impl(&mut self) -> Result<()> {
        use Response::*;

        let request_id = self.get_request_id();
        self.write(&Request::Terminate { request_id }).await?;

        match self.read_response().await? {
            Ok { response_id } => {
                Self::check_response_id(request_id, response_id)?;
                Result::Ok(())
            },
            PermissionDenied { response_id, reason } => {
                Self::check_response_id(request_id, response_id)?;
                Err(Error::PermissionDenied(reason))
            },
            _ => Err(Error::InvalidServerResponse(
                    "Expected Response: Ok or PermissionDenied"
                )),
        }
    }

    /// Request the master to terminate immediately.
    ///
    /// Return `Self` so that you can handle the error and reuse
    /// the `Connection`.
    pub async fn request_terminate(mut self) -> Result<(), (Error, Self)> {
        self.request_terminate_impl().await
            .map_err(|err| (err, self))
    }
}
