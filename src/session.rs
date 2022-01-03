#![forbid(unsafe_code)]

use super::{Connection, Error, Response, Result};

use std::io::ErrorKind;

enum EstablishedSessionState {
    Exited(Option<u32>),
    TtyAllocFail,
}

/// # Cancel safety
///
/// All methods of this struct is not cancellation safe.
#[derive(Debug)]
pub struct EstablishedSession {
    pub(super) conn: Connection,
    pub(super) session_id: u32,
}
impl EstablishedSession {
    fn check_session_id(&self, session_id: u32) -> Result<()> {
        if self.session_id != session_id {
            Err(Error::UnmatchedSessionId)
        } else {
            Ok(())
        }
    }

    /// Return None if TtyAllocFail, Some(...) if the process exited.
    async fn wait_impl(&mut self) -> Result<EstablishedSessionState> {
        use Response::*;

        let response = match self.conn.read_response().await {
            Result::Ok(response) => response,
            Err(err) => match &err {
                Error::IOError(io_err) if io_err.kind() == ErrorKind::UnexpectedEof => {
                    return Result::Ok(EstablishedSessionState::Exited(None))
                }
                _ => return Err(err),
            },
        };

        match response {
            TtyAllocFail { session_id } => {
                self.check_session_id(session_id)?;
                Result::Ok(EstablishedSessionState::TtyAllocFail)
            }
            ExitMessage {
                session_id,
                exit_value,
            } => {
                self.check_session_id(session_id)?;
                Result::Ok(EstablishedSessionState::Exited(Some(exit_value)))
            }
            response => Err(Error::InvalidServerResponse(
                "Expected Response TtyAllocFail or ExitMessage",
                response,
            )),
        }
    }

    /// Wait for session status to change
    ///
    /// Return `Self` on error so that you can handle the error and restart
    /// the operation.
    ///
    /// If the server close the connection without sending anything,
    /// this function would return `Ok(None)`.
    pub async fn wait(mut self) -> Result<SessionStatus, (Error, Self)> {
        use EstablishedSessionState::*;

        match self.wait_impl().await {
            Ok(Exited(exit_value)) => Ok(SessionStatus::Exited { exit_value }),
            Ok(TtyAllocFail) => Ok(SessionStatus::TtyAllocFail(self)),
            Err(err) => Err((err, self)),
        }
    }

    /// Return None if the socket is not readable,
    /// Some(None) if TtyAllocFail, Some(Some(...)) if the process exited.
    fn try_wait_impl(&mut self) -> Result<Option<EstablishedSessionState>> {
        use Response::*;

        let response = match self.conn.try_read_response() {
            Result::Ok(Some(response)) => response,
            Result::Ok(None) => return Result::Ok(None),
            Err(err) => match &err {
                Error::IOError(io_err) if io_err.kind() == ErrorKind::UnexpectedEof => {
                    return Result::Ok(Some(EstablishedSessionState::Exited(None)))
                }
                _ => return Err(err),
            },
        };

        match response {
            TtyAllocFail { session_id } => {
                self.check_session_id(session_id)?;
                Result::Ok(Some(EstablishedSessionState::TtyAllocFail))
            }
            ExitMessage {
                session_id,
                exit_value,
            } => {
                self.check_session_id(session_id)?;
                Result::Ok(Some(EstablishedSessionState::Exited(Some(exit_value))))
            }
            response => Err(Error::InvalidServerResponse(
                "Expected Response TtyAllocFail or ExitMessage",
                response,
            )),
        }
    }

    /// Since waiting for the remote child to exit is basically waiting for the socket
    /// to become readable, try_wait is basically polling for readable.
    ///
    /// If it is readable, then it would read the entire packet in blocking manner.
    /// While this is indeed a blocking call, it is unlikely to block since
    /// the ssh multiplex master most likely would send it using one write/send.
    ///
    /// And even if it does employ multiple write/send, these functions would just
    /// return immediately since the buffer for the unix socket is empty
    /// and should be big enough for one packet.
    ///
    /// If it is not readable, then it would return Ok(InProgress).
    pub fn try_wait(mut self) -> Result<TryWaitSessionStatus, (Error, Self)> {
        use EstablishedSessionState::*;

        match self.try_wait_impl() {
            Ok(Some(Exited(exit_value))) => Ok(TryWaitSessionStatus::Exited { exit_value }),
            Ok(Some(TtyAllocFail)) => Ok(TryWaitSessionStatus::TtyAllocFail(self)),
            Ok(None) => Ok(TryWaitSessionStatus::InProgress(self)),
            Err(err) => Err((err, self)),
        }
    }
}

#[derive(Debug)]
pub enum SessionStatus {
    /// Remote ssh server failed to allocate a tty, you can now return the tty
    /// to cooked mode.
    ///
    /// This arm includes `EstablishedSession` so that you can call `wait` on it
    /// again and retrieve the exit status and the underlying connection.
    TtyAllocFail(EstablishedSession),

    /// The process on the remote machine has exited with `exit_value`.
    Exited { exit_value: Option<u32> },
}

#[derive(Debug)]
pub enum TryWaitSessionStatus {
    /// Remote ssh server failed to allocate a tty, you can now return the tty
    /// to cooked mode.
    ///
    /// This arm includes `EstablishedSession` so that you can call `wait` on it
    /// again and retrieve the exit status and the underlying connection.
    TtyAllocFail(EstablishedSession),

    /// The process on the remote machine has exited with `exit_value`.
    Exited {
        exit_value: Option<u32>,
    },

    InProgress(EstablishedSession),
}
