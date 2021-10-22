use super::{Connection, Error, Response, Result};

use std::io::ErrorKind;

pub const UNEXPECTEDEOF: u32 = 255 << 8;

/// `EstablishedSession` contains the moved `Connection`, which once the session
/// has exited, you can get back this `Connection` and reused it.
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

    /// Return None if TtyAllocFail, Some(exit_value) if the process exited.
    async fn wait_impl(&mut self) -> Result<Option<u32>> {
        use Response::*;

        let response = match self.conn.read_response().await {
            Result::Ok(response) => response,
            Err(err) => match &err {
                Error::IOError(io_err) if io_err.kind() == ErrorKind::UnexpectedEof => {
                    return Result::Ok(Some(UNEXPECTEDEOF))
                }
                _ => return Err(err),
            },
        };

        match response {
            TtyAllocFail { session_id } => {
                self.check_session_id(session_id)?;
                Result::Ok(None)
            }
            ExitMessage {
                session_id,
                exit_value,
            } => {
                self.check_session_id(session_id)?;
                Result::Ok(Some(exit_value))
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
    /// this function would return `UNEXPECTEDEOF` as the exit code.
    pub async fn wait(mut self) -> Result<SessionStatus, (Error, Self)> {
        match self.wait_impl().await {
            Ok(Some(exit_value)) => Ok(SessionStatus::Exited { exit_value }),
            Ok(None) => Ok(SessionStatus::TtyAllocFail(self)),
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
    Exited { exit_value: u32 },
}
