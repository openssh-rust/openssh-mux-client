use serde::{Serialize, ser::Serializer};
use typed_builder::TypedBuilder;
use super::constants;

#[derive(Copy, Clone, Debug)]
pub enum Request<'a> {
    /// Response with `Response::Hello`.
    Hello { version: u32 },

    /// Server replied with `Response::Alive`.
    AliveCheck { request_id: u32 },

    /// For opening a new multiplexed session in passenger mode
    ///
    /// If successful, the server will reply with `Response::SessionOpened`.
    ///
    /// Otherwise it will reply with an error:
    ///  - `Response::PermissionDenied`;
    ///  - `Response::Failure`.
    ///
    /// The client then sends stdin, stdout and stderr fd.
    ///
    /// Once the server has received the fds, it will respond with `Response::Ok`
    /// indicating that the session is up. The client now waits for the
    /// session to end. When it does, the server will send `Response::ExitMessage`.
    ///
    /// Two additional cases that the client must cope with are it receiving
    /// a signal itself and the server disconnecting without sending an exit message.
    ///
    /// A master may also send a `Response::TtyAllocFail` before
    /// `Response::ExitMessage` if remote TTY allocation was unsuccessful.
    ///
    /// The client may use this to return its local tty to "cooked" mode.
    NewSession {
        request_id: u32,
        /// Must be set to empty string
        reserved:  &'static str,

        session: Session<'a>,
    },

    /// A server may reply with `Response::Ok`, `Response::RemotePort`,
    /// `Response::PermissionDenied`, or `Response::Failure`.
    /// 
    /// For dynamically allocated listen port the server replies with
    /// `Request::RemotePort`.
    OpenFwd {
        request_id: u32,
        fwd: Fwd<'a>,
    },

    /// A client may request the master to stop accepting new multiplexing requests
    /// and remove its listener socket.
    ///
    /// A server may reply with `Response::Ok`, `Response::PermissionDenied` or
    /// `Response::Failure`.
    StopListening { request_id: u32 },

    /// A client may request that a master terminate immediately.
    /// Server may response with `Response::Ok` or `Response::PermissionDenied`.
    Terminate { request_id: u32 },
}
impl<'a> Serialize for Request<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use Request::*;
        use constants::*;

        match self {
            Hello { version } =>
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_MSG_HELLO,
                    "Hello",
                    version
                ),
            AliveCheck { request_id } =>
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_C_ALIVE_CHECK,
                    "AliveCheck",
                    request_id
                ),
            NewSession { request_id, reserved, session } => {
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_C_NEW_SESSION,
                    "NewSession",
                    &(*request_id, *reserved, *session)
                )
            },
            OpenFwd { request_id, fwd } =>
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_C_OPEN_FWD,
                    "OpenFwd",
                    &(*request_id, *fwd)
                ),
            StopListening { request_id } =>
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_C_STOP_LISTENING,
                    "StopListening",
                    request_id
                ),
            Terminate { request_id } =>
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_C_TERMINATE,
                    "Terminate",
                    request_id
                ),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, TypedBuilder)]
pub struct Session<'a> {
    #[builder(default = false)]
    pub tty: bool,

    #[builder(default = false)]
    pub x11_forwarding: bool,

    #[builder(default = false)]
    pub agent: bool,

    #[builder(default = false)]
    pub subsystem: bool,

    /// Set to `0xffffffff`(`char::MAX`) to disable escape character
    #[builder(default = char::MAX)]
    pub escape_ch: char,

    /// Generally set to `$TERM`.
    pub term: &'a str,
    pub cmd: &'a str,

    pub env: Option<&'a [&'a str]>,
}

#[derive(Copy, Clone, Debug)]
pub enum Fwd<'a> {
    Local {
        listen_socket: Socket<'a>,
        connect_socket: Socket<'a>,
    },
    Remote {
        listen_socket: Socket<'a>,
        connect_socket: Socket<'a>,
    },
    Dynamic {
        listen_socket: Socket<'a>,
    },
}
impl<'a> Serialize for Fwd<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use Fwd::*;

        match self {
            Local { listen_socket, connect_socket } => {
                serializer.serialize_newtype_variant(
                    "Fwd",
                    constants::MUX_FWD_LOCAL,
                    "Local",
                    &(*listen_socket, *connect_socket)
                )
            },
            Remote { listen_socket, connect_socket } => {
                serializer.serialize_newtype_variant(
                    "Fwd",
                    constants::MUX_FWD_REMOTE,
                    "Remote",
                    &(*listen_socket, *connect_socket)
                )
            },
            Dynamic { listen_socket } => {
                serializer.serialize_newtype_variant(
                    "Fwd",
                    constants::MUX_FWD_DYNAMIC,
                    "Dynamic",
                    listen_socket
                )
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Socket<'a> {
    UnixSocket(&'a str),
    TcpSocket(&'a str, u32),
}
impl<'a> Serialize for Socket<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use Socket::*;

        let unix_socket_port = -2;

        let value = match self {
            UnixSocket(path) => (*path, unix_socket_port as u32),
            TcpSocket(host, port) => (*host, *port),
        };

        value.serialize(serializer)
    }
}
