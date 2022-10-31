use std::sync::Arc;

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::{
    proxy_client::channel::{ChannelData, MpscBytesChannel},
    Error,
};

const LEN: usize = 64;
const BITARRAY_LEN: usize = LEN / (usize::BITS as usize);

type ChannelDataArena = concurrent_arena::Arena<ChannelData, BITARRAY_LEN, LEN>;
pub(super) type ChannelDataArenaArc = concurrent_arena::ArenaArc<ChannelData, BITARRAY_LEN, LEN>;

#[derive(Debug, Default, Clone)]
pub(super) struct SharedData(Arc<SharedDataInner>);

impl SharedData {
    pub(super) fn get_write_channel(&self) -> &MpscBytesChannel {
        &self.0.write_channel
    }

    pub(super) fn insert_channel_data(&self, channel_data: ChannelData) -> ChannelDataArenaArc {
        self.0.channel_data_arena.insert(channel_data)
    }

    pub(super) fn remove_channel_data(&self, slot: u32) -> Result<ChannelDataArenaArc, Error> {
        self.0
            .channel_data_arena
            .remove(slot)
            .ok_or(Error::InvalidRecipientChannel(slot))
    }

    pub(super) fn get_channel_data(&self, slot: u32) -> Result<ChannelDataArenaArc, Error> {
        self.0
            .channel_data_arena
            .get(slot)
            .ok_or(Error::InvalidRecipientChannel(slot))
    }

    pub(super) fn get_read_task_shutdown_notifier(&self) -> &Notify {
        &self.0.read_task_shutdown_notifier
    }

    pub(super) fn get_cancellation_token(&self) -> &CancellationToken {
        &self.0.cancellation_token
    }
}

impl Drop for SharedData {
    fn drop(&mut self) {
        if Arc::strong_count(&self.0) == 3 {
            // There are only three references to `Arc` now:
            //  - This reference to `Arc`
            //  - The reference in read_task
            //  - The reference in write_task
            //
            // which means that we should request shutdown now.
            //
            // Once write_channel is marked as eof, write task
            // would exit as soon as all data is flushed.
            //
            // Then it would notify read task to also shutdown.
            self.get_write_channel().mark_eof();
        }
    }
}

#[derive(Debug, Default)]
struct SharedDataInner {
    write_channel: MpscBytesChannel,
    channel_data_arena: ChannelDataArena,

    read_task_shutdown_notifier: Notify,

    cancellation_token: CancellationToken,
}
