use std::{num::NonZeroUsize, pin::Pin};

use hash_hasher::HashedMap as HashMap;
use tokio::{io::AsyncRead, pin, spawn, task::JoinHandle};

use crate::{
    proxy_client::{ChannelDataArenaArc, SharedData},
    Error,
};

#[derive(Debug)]
enum PendingRequests {
    Pending {
        pending: NonZeroUsize,
        /// Has any request failed
        has_failed: bool,
    },
    Done,
}

#[derive(Debug)]
struct ChannelIngoingData {
    outgoing_data: ChannelDataArenaArc,

    /// Once this get into zero and `outgoing_data.receivers_count != 0`,
    /// then read task should send `extend_window_size_packet`.
    receiver_win_size: u32,

    /// Check [`super::channel::ChannelState`] for doc.
    extend_window_size_packet: [u8; 14],

    pending_requests: PendingRequests,
}

pub(super) fn create_read_task<R>(rx: R, shared_data: SharedData) -> JoinHandle<Result<(), Error>>
where
    R: AsyncRead + Send + 'static,
{
    async fn inner(
        rx: Pin<&mut (dyn AsyncRead + Send)>,
        shared_data: SharedData,
    ) -> Result<(), Error> {
        todo!()
    }

    spawn(async move {
        pin!(rx);

        inner(rx, shared_data).await
    })
}
