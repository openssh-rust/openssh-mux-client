#![forbid(unsafe_code)]

use super::{Connection, Error, ErrorExt, Response, Result};

use std::io::ErrorKind;

enum EstablishedSessionState {
    Exited(Option<u32>),
    TtyAllocFail,
}

/// NOTE that once `EstablishedSession` is dropped, any data written to
/// `stdin` will not be send to the remote process and
/// `stdout` and `stderr` would eof immediately.
///
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
            response => Err(Error::invalid_server_response(
                &"TtyAllocFail or ExitMessage",
                &response,
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
