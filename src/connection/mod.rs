mod constants;
mod error;
mod raw_connection;
mod request;
mod response;
mod session;

pub mod default_config;

use raw_connection::RawConnection;
use request::Fwd;
use request::Request;

use core::convert::AsRef;
use core::mem;
use core::num::{NonZeroU32, Wrapping};
use std::path::Path;

use serde::{Deserialize, Serialize};
use ssh_mux_format::{from_bytes, Serializer};

use std::os::unix::io::RawFd;

pub use error::Error;
pub use response::Response;
pub type Result<T, Err = Error> = std::result::Result<T, Err>;

pub use request::{Session, Socket};
pub use session::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ForwardType {
    Local,
    Remote,
}

/// # Cancel safety
///
/// All methods of this struct is not cancellation safe.
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
        T: Deserialize<'a>,
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
        self.write(&Request::Hello {
            version: constants::SSHMUX_VER,
        })
        .await?;

        let response = self.read_response().await?;
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

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self {
            raw_conn: RawConnection::connect(path).await?,
            serializer: Serializer::new(),
            buffer: Vec::with_capacity(mem::size_of::<Response>()),
            request_id: Wrapping(0),
        }
        .exchange_hello()
        .await
    }

    /// Send a ping to the server and return pid of the ssh mux server
    /// if it is still alive.
    pub async fn send_alive_check(&mut self) -> Result<NonZeroU32> {
        let request_id = self.get_request_id();

        self.write(&Request::AliveCheck { request_id }).await?;

        let response = self.read_response().await?;
        if let Response::Alive {
            response_id,
            server_pid,
        } = response
        {
            Self::check_response_id(request_id, response_id)?;
            NonZeroU32::new(server_pid).ok_or(Error::InvalidPid)
        } else {
            Err(Error::InvalidServerResponse(
                "Expected Response::Alive",
                response,
            ))
        }
    }

    /// Return session_id
    async fn open_new_session_impl(
        &mut self,
        session: &Session<'_>,
        fds: &[RawFd; 3],
    ) -> Result<u32> {
        use Response::*;

        let request_id = self.get_request_id();

        self.write(&Request::NewSession {
            request_id,
            reserved: "",
            session,
        })
        .await?;
        for fd in fds {
            self.raw_conn.send_with_fds(&[0], &[*fd])?;
        }

        let session_id = match self.read_response().await? {
            SessionOpened {
                response_id,
                session_id,
            } => {
                Self::check_response_id(request_id, response_id)?;
                session_id
            }
            PermissionDenied {
                response_id,
                reason,
            } => {
                Self::check_response_id(request_id, response_id)?;
                return Err(Error::PermissionDenied(reason));
            }
            Failure {
                response_id,
                reason,
            } => {
                Self::check_response_id(request_id, response_id)?;
                return Err(Error::RequestFailure(reason));
            }
            response => {
                return Err(Error::InvalidServerResponse(
                    "Expected Response: SessionOpened, PermissionDenied or Failure",
                    response,
                ))
            }
        };

        Result::Ok(session_id)
    }

    /// Opens a new session.
    ///
    /// Consumes `self` so that users would not be able to create multiple sessions
    /// or perform other operations during the session that might complicates the
    /// handling of packets received from the ssh mux server.
    ///
    /// Two additional cases that the client must cope with are it receiving
    /// a signal itself (from the ssh mux server) and the server disconnecting
    /// without sending an exit message.
    ///
    /// The return value `EstablishedSession` will contain the moved `self`, which once
    /// the session has exited, you can get back this `Connection` and reused it.
    ///
    /// Return `Self` on `Err` so that you can handle the error
    /// and reuse the `Connection`.
    pub async fn open_new_session(
        mut self,
        session: &Session<'_>,
        fds: &[RawFd; 3],
    ) -> Result<EstablishedSession, (Error, Self)> {
        match self.open_new_session_impl(session, fds).await {
            Ok(session_id) => Ok(EstablishedSession {
                conn: self,
                session_id,
            }),
            Err(err) => Err((err, self)),
        }
    }

    /// Request for local/remote port forwarding.
    ///
    /// # Warning
    ///
    /// Local port forwarding hasn't been tested yet.
    pub async fn request_port_forward(
        &mut self,
        forward_type: ForwardType,
        listen_socket: &Socket<'_>,
        connect_socket: &Socket<'_>,
    ) -> Result<()> {
        use ForwardType::*;
        use Response::*;

        let fwd = match forward_type {
            Local => Fwd::Local {
                listen_socket,
                connect_socket,
            },
            Remote => Fwd::Remote {
                listen_socket,
                connect_socket,
            },
        };
        let fwd = &fwd;

        let request_id = self.get_request_id();
        self.write(&Request::OpenFwd { request_id, fwd }).await?;

        match self.read_response().await? {
            Ok { response_id } => Self::check_response_id(request_id, response_id),
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

    /// **UNTESTED** Return remote port opened for dynamic forwarding.
    pub async fn request_dynamic_forward(
        &mut self,
        listen_socket: &Socket<'_>,
    ) -> Result<NonZeroU32> {
        use Response::*;

        let fwd = Fwd::Dynamic { listen_socket };
        let fwd = &fwd;

        let request_id = self.get_request_id();
        self.write(&Request::OpenFwd { request_id, fwd }).await?;

        match self.read_response().await? {
            RemotePort {
                response_id,
                remote_port,
            } => {
                Self::check_response_id(request_id, response_id)?;
                NonZeroU32::new(remote_port).ok_or(Error::InvalidPort)
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
                "Expected Response: RemotePort, PermissionDenied or Failure",
                response,
            )),
        }
    }

    /// Request the master to stop accepting new multiplexing requests
    /// and remove its listener socket.
    pub async fn request_stop_listening(&mut self) -> Result<()> {
        use Response::*;

        let request_id = self.get_request_id();
        self.write(&Request::StopListening { request_id }).await?;

        match self.read_response().await? {
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

    async fn request_terminate_impl(&mut self) -> Result<()> {
        use Response::*;

        let request_id = self.get_request_id();
        self.write(&Request::Terminate { request_id }).await?;

        match self.read_response().await? {
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
            response => Err(Error::InvalidServerResponse(
                "Expected Response: Ok or PermissionDenied",
                response,
            )),
        }
    }

    /// (test failed) Request the master to terminate immediately.
    ///
    /// Return `Self` on `Err` so that you can handle the error and reuse
    /// the `Connection`.
    pub async fn request_terminate(mut self) -> Result<(), (Error, Self)> {
        self.request_terminate_impl()
            .await
            .map_err(|err| (err, self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io;
    use std::os::unix::io::AsRawFd;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_pipe::{pipe, PipeRead, PipeWrite};
    
    const PATH: &str = "/tmp/openssh-mux-client-test.socket";

    macro_rules! run_test {
        ( $test_name:ident, $func:ident ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() {
                let conn = Connection::connect(PATH).await.unwrap();

                $func(conn).await;
            }
        };
    }

    async fn test_connect_impl(_conn: Connection) {}
    run_test!(test_unordered_connect, test_connect_impl);

    async fn test_alive_check_impl(mut conn: Connection) {
        let expected_pid = env::var("ControlMasterPID").unwrap();
        let expected_pid: u32 = expected_pid.parse().unwrap();

        let actual_pid = conn.send_alive_check().await.unwrap().get();
        assert_eq!(expected_pid, actual_pid);
    }
    run_test!(test_unordered_alive_check, test_alive_check_impl);

    async fn test_roundtrip<const SIZE: usize>(
        stdios: &mut (PipeWrite, PipeRead),
        data: &'static [u8; SIZE],
    ) {
        stdios.0.write_all(data).await.unwrap();

        let mut buffer = [0 as u8; SIZE];
        stdios.1.read_exact(&mut buffer).await.unwrap();

        assert_eq!(data, &buffer);
    }

    async fn create_remote_process(
        conn: Connection,
        cmd: &str,
    ) -> (EstablishedSession, (PipeWrite, PipeRead)) {
        let session = Session::builder().cmd(cmd).build();

        // pipe() returns (PipeRead, PipeWrite)
        let (stdin_read, stdin_write) = pipe().unwrap();
        let (stdout_read, stdout_write) = pipe().unwrap();

        let established_session = conn
            .open_new_session(
                &session,
                &[
                    stdin_read.as_raw_fd(),
                    stdout_write.as_raw_fd(),
                    io::stderr().as_raw_fd(),
                ],
            )
            .await
            .unwrap();

        (established_session, (stdin_write, stdout_read))
    }

    async fn test_open_new_session_impl(conn: Connection) {
        let (established_session, mut stdios) = create_remote_process(conn, "/bin/cat").await;

        // All test data here must end with '\n', otherwise cat would output nothing
        // and the test would hang forever.

        test_roundtrip(&mut stdios, &b"0134131dqwdqdx13as\n").await;
        test_roundtrip(&mut stdios, &b"Whats' Up?\n").await;

        drop(stdios);

        let session_status = established_session.wait().await.unwrap();
        assert_matches!(
            session_status,
            SessionStatus::Exited { exit_value, .. }
                if exit_value == 0
        );
    }
    run_test!(test_unordered_open_new_session, test_open_new_session_impl);

    async fn test_socket_forward_impl(mut conn: Connection) {
        let path = "/tmp/openssh-local-forward.socket";

        let output_listener = TcpListener::bind(("127.0.0.1", 1234)).await.unwrap();

        eprintln!("Requesting port forward");
        conn.request_port_forward(
            ForwardType::Remote,
            &Socket::UnixSocket { path },
            &Socket::TcpSocket {
                port: NonZeroU32::new(1234).unwrap(),
                host: "127.0.0.1",
            },
        )
        .await
        .unwrap();

        eprintln!("Creating remote process");
        let cmd = format!("/usr/bin/socat OPEN:/data,rdonly UNIX-CONNECT:{}", path);
        let (established_session, stdios) = create_remote_process(conn, &cmd).await;

        eprintln!("Waiting for connection");
        let (mut output, _addr) = output_listener.accept().await.unwrap();

        // All test data here must end with '\n', otherwise remote
        // command would output nothing and the test would hang forever.

        eprintln!("Reading");

        const DATA: &[u8] = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n".as_bytes();

        let mut buffer = [0 as u8; DATA.len()];
        output.read_exact(&mut buffer).await.unwrap();

        assert_eq!(DATA, &buffer);

        drop(output);
        drop(output_listener);
        drop(stdios);

        eprintln!("Waiting for session to end");
        let session_status = established_session.wait().await.unwrap();
        assert_matches!(
            session_status,
            SessionStatus::Exited { exit_value, .. }
                if exit_value == 0
        );
    }
    run_test!(test_unordered_socket_forward, test_socket_forward_impl);

    async fn test_request_stop_listing_impl(mut conn: Connection) {
        conn.request_stop_listening().await.unwrap();

        eprintln!("Verify that existing connection is still usable.");
        test_open_new_session_impl(conn).await;

        eprintln!(
            "Verify that after the last connection is dropped, the multiplex server \
            indeed shutdown."
        );
        assert_matches!(Connection::connect(PATH).await, Err(_));
    }
    run_test!(test_request_stop_listing, test_request_stop_listing_impl);

    async fn test_request_terminate_impl(conn: Connection) {
        conn.request_terminate().await.unwrap();

        eprintln!(
            "Verify that the multiplex server indeed terminated."
        );
        assert_matches!(Connection::connect(PATH).await, Err(_));
    }
    run_test!(test_request_terminate, test_request_terminate_impl);
}
