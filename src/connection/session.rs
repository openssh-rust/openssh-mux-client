use super::{Connection, Error, Result, Response};

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

    /// Wait for session status to change
    pub async fn wait(mut self) -> Result<SessionStatus> {
        use Response::*;

        match self.conn.read_response().await? {
            TtyAllocFail { session_id } => {
                self.check_session_id(session_id)?;
                Result::Ok(SessionStatus::TtyAllocFail(self))
            },
            ExitMessage { session_id, exit_value } => {
                self.check_session_id(session_id)?;
                Result::Ok(SessionStatus::Exited {
                    conn: self.conn,
                    exit_value,
                })
            },
            _ => Err(Error::InvalidServerResponse(
                "Expected Response TtyAllocFail or ExitMessage"
            ))
        }
    }
}

pub enum SessionStatus {
    TtyAllocFail(EstablishedSession),
    Exited {
        conn: Connection,
        exit_value: u32,
    },
}
