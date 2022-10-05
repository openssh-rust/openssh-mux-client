use super::channel::{ChannelData, MpscBytesChannel};

const LEN: usize = 64;
const BITARRAY_LEN: usize = LEN / (usize::BITS as usize);

type ChannelDataArena = concurrent_arena::Arena<ChannelData, BITARRAY_LEN, LEN>;
pub(super) type ChannelDataArenaArc = concurrent_arena::ArenaArc<ChannelData, BITARRAY_LEN, LEN>;

#[derive(Debug)]
pub(super) struct SharedData {
    pub(super) write_channel: MpscBytesChannel,
    pub(super) channel_data_arena: ChannelDataArena,
}
