use std::{num::NonZeroUsize, pin::Pin};

use scopeguard::defer;
use tokio::{io::AsyncWrite, pin, spawn, task::JoinHandle};
use tokio_io_utility::{write_all_bytes, ReusableIoSlices};

use crate::{proxy_client::SharedData, Error};

pub(super) fn create_write_task<W>(
    tx: W,
    shared_data: SharedData,
    reusable_io_slice_cap: NonZeroUsize,
) -> JoinHandle<Result<(), Error>>
where
    W: AsyncWrite + Send + 'static,
{
    spawn(async move {
        pin!(tx);

        create_write_task_inner(tx, shared_data, reusable_io_slice_cap).await
    })
}

async fn create_write_task_inner(
    mut tx: Pin<&mut (dyn AsyncWrite + Send)>,
    shared_data: SharedData,
    reusable_io_slice_cap: NonZeroUsize,
) -> Result<(), Error> {
    let write_channel = shared_data.get_write_channel();
    let mut reusable_io_slice = ReusableIoSlices::new(reusable_io_slice_cap);

    let mut buffer = Vec::new();

    defer! {
        shared_data.get_read_task_shutdown_notifier().notify_one();
    }

    loop {
        write_channel.wait_for_data(&mut buffer).await;
        if buffer.is_empty() {
            // Eof
            break;
        }

        write_all_bytes(tx.as_mut(), &mut buffer, &mut reusable_io_slice).await?;
    }

    Ok(())
}
