use super::{Connection, Error, Result, Response};

/// `EstablishedSession` contains the moved `Connection`, which once the session
/// has exited, you can get back this `Connection` and reused it.
#[derive(Debug)]
pub struct EstablishedSession {
    pub(super) conn: Connection,
    pub(super) session_id: u32,
}
impl EstablishedSession {
    /// Wait for session status to change
    ///
    /// Return `Self` on error so that you can handle the error and restart
    /// the operation.
    pub async fn wait(mut self) -> Result<SessionStatus, (Error, Self)> {
        use Response::*;

        let response = match self.conn.read_response().await {
            Result::Ok(response) => response,
            Err(err) => return Err((err, self)),
        };

        match response {
            TtyAllocFail { session_id } => {
                if self.session_id != session_id {
                    Err((Error::UnmatchedSessionId, self))
                } else {
                    Result::Ok(SessionStatus::TtyAllocFail(self))
                }
            },
            ExitMessage { session_id, exit_value } => {
                if self.session_id != session_id {
                    Err((Error::UnmatchedSessionId, self))
                } else {
                    Result::Ok(SessionStatus::Exited {
                        conn: self.conn,
                        exit_value,
                    })
                }
            },
            _ => Err((
                Error::InvalidServerResponse(
                    "Expected Response TtyAllocFail or ExitMessage"
                ),
                self
            ))
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
