use serde::{Serialize, ser::Serializer};
use super::constants;

#[derive(Copy, Clone, Debug)]
pub enum Request<'a, 'b> {
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
        session: Session<'a, 'b>,
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
impl<'a, 'b> Serialize for Request<'a, 'b> {
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
            NewSession { request_id, session } => {
                serializer.serialize_newtype_variant(
                    "Request",
                    MUX_C_NEW_SESSION,
                    "NewSession",
                    &(*request_id, *session)
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

#[derive(Copy, Clone, Debug, Serialize)]
pub struct Session<'a, 'b> {
    /// Must be set to empty string
    reserved:  &'static str,

    tty: bool,
    x11_forwarding: bool,
    agent: bool,
    subsystem: bool,

    /// Set to `0xffffffff` to disable escape character
    escape_ch: char,
    /// Generally set to `$TERM`.
    term: &'a str,
    cmd: &'a str,
    env: Option<&'a [&'b str]>,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct Fwd<'a> {
    fwd_type: ForwardingType,
    listen_socket: Socket<'a>,
    connect_socket: Socket<'a>,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum ForwardingType {
    Local   = super::constants::MUX_FWD_LOCAL,
    Remote  = super::constants::MUX_FWD_REMOTE,
    Dynamic = super::constants::MUX_FWD_DYNAMIC,
}
impl Serialize for ForwardingType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit_variant(
            "ForwardingType",
            *self as u32,
            "",
        )
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
