use std::sync::Arc;

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
}

#[derive(Debug, Default)]
struct SharedDataInner {
    write_channel: MpscBytesChannel,
    channel_data_arena: ChannelDataArena,
}
