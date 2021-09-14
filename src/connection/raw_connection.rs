use core::convert::AsRef;
use std::path::Path;
use std::io;

use tokio::net::UnixStream;

use std::os::unix::io::AsRawFd;
use passfd::FdPassingExt;

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

    pub fn send_fds<T: AsRawFd>(&self, vals: &[T]) -> Result<()> {
        let stream_fd = AsRawFd::as_raw_fd(&self.stream);
        for val in vals {
            FdPassingExt::send_fd(&stream_fd, AsRawFd::as_raw_fd(val))?;
        }
        Ok(())
    }

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            stream: UnixStream::connect(path).await?,
        })
    }
}
