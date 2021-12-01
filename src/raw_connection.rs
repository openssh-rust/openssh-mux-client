use std::io;
use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use sendfd::SendWithFd;
use std::os::unix::io::RawFd;

use super::Result;

/// # Cancel safety
///
/// All methods of this struct is not cancellation safe.
#[derive(Debug)]
pub struct RawConnection {
    stream: UnixStream,
}
impl RawConnection {
    pub fn into_std(self) -> Result<std::os::unix::net::UnixStream> {
        Ok(self.stream.into_std()?)
    }

    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.stream.write_all(bytes).await?;

        Ok(())
    }

    pub async fn read(&mut self, bytes: &mut [u8]) -> Result<()> {
        self.stream.read_exact(bytes).await?;

        Ok(())
    }

    /// If it is readable, then the entire packet can be read in blocking manner.
    ///
    /// While this is indeed a blocking call, it is unlikely to block since ssh mux
    /// master most likely would send it using one write/send.
    ///
    /// Even if it does employ multiple write/send, these functions would just return
    /// immediately since the buffer for the unix socket is empty and should be big
    /// enough for one message.
    ///
    /// If it is not readable, then it would return Ok(None).
    pub fn try_read(&self, bytes: &mut [u8]) -> Result<Option<()>> {
        let mut nread = 0;

        while nread < bytes.len() {
            match self.stream.try_read(&mut bytes[nread..]) {
                Ok(n) => {
                    if n == 0 {
                        let err: io::Error = io::ErrorKind::UnexpectedEof.into();
                        return Err(err.into());
                    }

                    nread += n;
                }
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e.into());
                    } else if nread == 0 {
                        return Ok(None);
                    }
                }
            }
        }

        Ok(Some(()))
    }

    /// Send fds with "\0"
    pub async fn send_with_fds(&self, fds: &[RawFd]) -> Result<()> {
        let byte = &[0];

        loop {
            self.stream.writable().await?;

            match SendWithFd::send_with_fd(&self.stream, byte, fds) {
                Ok(n) => {
                    if n == 1 {
                        break Ok(());
                    }
                }
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        break Err(e.into());
                    }
                }
            }
        }
    }

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            stream: UnixStream::connect(path).await?,
        })
    }
}
