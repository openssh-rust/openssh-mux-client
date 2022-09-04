#![forbid(unsafe_code)]

use super::Result;

use std::io;
use std::os::unix::io::RawFd;
use std::path::Path;

use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use sendfd::SendWithFd;

/// # Cancel safety
///
/// All methods of this struct is not cancellation safe.
#[derive(Debug)]
pub struct RawConnection {
    pub(crate) stream: UnixStream,
}
impl RawConnection {
    pub fn into_std(self) -> Result<std::os::unix::net::UnixStream> {
        Ok(self.stream.into_std()?)
    }

    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.stream.write_all(bytes).await?;

        Ok(())
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
