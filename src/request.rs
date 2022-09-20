#![forbid(unsafe_code)]

use super::{constants, default_config, NonZeroByteSlice};

use std::{borrow::Cow, path::Path};

use serde::{Serialize, Serializer};
use typed_builder::TypedBuilder;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Request<'a> {
    /// Response with `Response::Hello`.
    Hello { version: u32 },

    /// Server replied with `Response::Alive`.
    AliveCheck { request_id: u32 },

    /// For opening a new multiplexed session in passenger mode,
    /// send this variant and then sends stdin, stdout and stderr fd.
    ///
    /// If successful, the server will reply with `Response::SessionOpened`.
    ///
    /// Otherwise it will reply with an error:
    ///  - `Response::PermissionDenied`;
    ///  - `Response::Failure`.
    ///
    /// The client now waits for the session to end. When it does, the server
    /// will send `Response::ExitMessage`.
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
        session: SessionZeroCopy,
    },

    /// A server may reply with `Response::Ok`, `Response::RemotePort`,
    /// `Response::PermissionDenied`, or `Response::Failure`.
    ///
    /// For dynamically allocated listen port the server replies with
    /// `Request::RemotePort`.
    OpenFwd { request_id: u32, fwd: &'a Fwd<'a> },

    /// A client may request the master to stop accepting new multiplexing requests
    /// and remove its listener socket.
    ///
    /// A server may reply with `Response::Ok`, `Response::PermissionDenied` or
    /// `Response::Failure`.
    StopListening { request_id: u32 },
}
impl<'a> Serialize for Request<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use constants::*;
        use Request::*;

        match self {
            Hello { version } => {
                serializer.serialize_newtype_variant("Request", MUX_MSG_HELLO, "Hello", version)
            }
            AliveCheck { request_id } => serializer.serialize_newtype_variant(
                "Request",
                MUX_C_ALIVE_CHECK,
                "AliveCheck",
                request_id,
            ),
            NewSession {
                request_id,
                session,
            } => serializer.serialize_newtype_variant(
                "Request",
                MUX_C_NEW_SESSION,
                "NewSession",
                &(*request_id, "", *session),
            ),
            OpenFwd { request_id, fwd } => serializer.serialize_newtype_variant(
                "Request",
                MUX_C_OPEN_FWD,
                "OpenFwd",
                &(*request_id, *fwd),
            ),
            StopListening { request_id } => serializer.serialize_newtype_variant(
                "Request",
                MUX_C_STOP_LISTENING,
                "StopListening",
                request_id,
            ),
        }
    }
}

/// Zero copy version of [`Session`]
#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct SessionZeroCopy {
    pub tty: bool,

    pub x11_forwarding: bool,

    pub agent: bool,

    pub subsystem: bool,

    pub escape_ch: char,

    /// len of [`Session::term`].
    pub term_len: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, TypedBuilder)]
#[builder(doc)]
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
    #[builder(default_code = r#"Cow::Borrowed(default_config::get_term())"#)]
    pub term: Cow<'a, NonZeroByteSlice>,
    pub cmd: Cow<'a, NonZeroByteSlice>,
}

#[derive(Copy, Clone, Debug)]
pub enum Fwd<'a> {
    Local {
        listen_socket: &'a Socket<'a>,
        connect_socket: &'a Socket<'a>,
    },
    Remote {
        listen_socket: &'a Socket<'a>,
        connect_socket: &'a Socket<'a>,
    },
    Dynamic {
        listen_socket: &'a Socket<'a>,
    },
}
impl<'a> Serialize for Fwd<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use Fwd::*;

        match self {
            Local {
                listen_socket,
                connect_socket,
            } => serializer.serialize_newtype_variant(
                "Fwd",
                constants::MUX_FWD_LOCAL,
                "Local",
                &(*listen_socket, *connect_socket),
            ),
            Remote {
                listen_socket,
                connect_socket,
            } => serializer.serialize_newtype_variant(
                "Fwd",
                constants::MUX_FWD_REMOTE,
                "Remote",
                &(*listen_socket, *connect_socket),
            ),
            Dynamic { listen_socket } => serializer.serialize_newtype_variant(
                "Fwd",
                constants::MUX_FWD_DYNAMIC,
                "Dynamic",
                &(
                    *listen_socket,
                    Socket::UnixSocket {
                        path: Path::new("").into(),
                    },
                ),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Socket<'a> {
    UnixSocket { path: Cow<'a, Path> },
    TcpSocket { port: u32, host: Cow<'a, str> },
}
impl Socket<'_> {
    pub(crate) fn as_serializable(&self) -> (Cow<'_, str>, u32) {
        use Socket::*;

        let unix_socket_port: i32 = -2;

        match self {
            // Serialize impl for Path calls to_str and ret err if failed,
            // so calling to_string_lossy is OK as it does not break backward
            // compatibility.
            UnixSocket { path } => (path.to_string_lossy(), unix_socket_port as u32),
            TcpSocket { port, host } => (Cow::Borrowed(host), *port),
        }
    }
}
impl<'a> Serialize for Socket<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_serializable().serialize(serializer)
    }
}
