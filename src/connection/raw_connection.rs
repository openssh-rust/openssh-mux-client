use core::convert::AsRef;
use std::path::Path;
use std::io;

use tokio::net::UnixStream;

use std::os::unix::io::{AsRawFd, RawFd};
use sendfd::SendWithFd;

use super::Result;

#[derive(Debug)]
pub struct RawConnection {
    stream: UnixStream,
}
impl RawConnection {
    pub async fn write(&self, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            self.stream.writable().await?;

            match self.stream.try_write(bytes) {
                Ok(n) => {
                    bytes = &bytes[n..];
                },
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn read(&self, mut bytes: &mut [u8]) -> Result<()> {
        while !bytes.is_empty() {
            self.stream.readable().await?;

            match self.stream.try_read(bytes) {
                Ok(n) => {
                    bytes = &mut bytes[n..];
                },
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(())
    }

    pub fn send_with_fds(&self, bytes: &[u8], vals: &[RawFd]) -> Result<()> {
        let stream_fd = AsRawFd::as_raw_fd(&self.stream);
        SendWithFd::send_with_fd(&stream_fd, bytes, vals)?;
        Ok(())
    }

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            stream: UnixStream::connect(path).await?,
        })
    }
}
