use super::{Connection, Error, Result, Response};

/// `EstablishedSession` contains the moved `Connection`, which once the session
/// has exited, you can get back this `Connection` and reused it.
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

        match self.conn.read_response().await? {
            TtyAllocFail { session_id } => {
                self.check_session_id(session_id)?;
                Result::Ok(None)
            },
            ExitMessage { session_id, exit_value } => {
                self.check_session_id(session_id)?;
                Result::Ok(Some(exit_value))
            },
            response =>
                Err(Error::InvalidServerResponse(
                    "Expected Response TtyAllocFail or ExitMessage",
                    response
                )),
        }
    }

    /// Wait for session status to change
    ///
    /// Return `Self` on error so that you can handle the error and restart
    /// the operation.
    pub async fn wait(mut self) -> Result<SessionStatus, (Error, Self)> {
        match self.wait_impl().await {
            Ok(Some(exit_value)) =>
                Ok(SessionStatus::Exited {
                    conn: self.conn,
                    exit_value,
                }),
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
    Exited {
        /// Return the connection so that you can reuse it.
        conn: Connection,
        exit_value: u32,
    },
}
