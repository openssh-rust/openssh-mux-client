use std::num::NonZeroUsize;

use openssh_proxy_client_error::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    task::JoinHandle,
};

mod channel;

mod shared_data;
use shared_data::{ChannelDataArenaArc, SharedData};

mod read_task;
use read_task::create_read_task;

mod write_task;
use write_task::create_write_task;

#[derive(Debug)]
pub struct ProxyClient {
    shared_data: SharedData,
    read_task: JoinHandle<Result<(), Error>>,
    write_task: JoinHandle<Result<(), Error>>,
}

impl ProxyClient {
    /// * `reusable_io_slice_cap` - determines how many `Bytes` can be sent
    ///   in one syscall to reduce overhead.
    pub fn new<R, W>(rx: R, tx: W, reusable_io_slice_cap: NonZeroUsize) -> Self
    where
        R: AsyncRead + Send + 'static,
        W: AsyncWrite + Send + 'static,
    {
        let shared_data = SharedData::default();

        Self {
            write_task: create_write_task(tx, shared_data.clone(), reusable_io_slice_cap),
            read_task: create_read_task(rx, shared_data.clone()),
            shared_data,
        }
    }

    pub async fn close(self) -> Result<(), Error> {
        drop(self.shared_data);

        self.read_task.await??;
        self.write_task.await??;

        Ok(())
    }
}
