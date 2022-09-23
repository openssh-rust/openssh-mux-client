use compact_str::CompactString;
use serde::Deserialize;

pub(crate) mod error;
pub use error::*;

mod exit_status;
pub(crate) use exit_status::*;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenConfirmation {
    pub(crate) sender_channel: u32,
    pub(crate) init_win_size: u32,
    pub(crate) max_packet_size: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct BytesAdjust {
    pub(crate) bytes_to_add: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelExtendedData {
    pub(crate) data_type: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelRequest<T> {
    pub(crate) request_type: CompactString,
    pub(crate) want_reply: bool,
    pub(crate) data: T,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelOpen<T> {
    pub(crate) channel_type: CompactString,
    pub(crate) sender_channel: u32,
    pub(crate) init_win_size: u32,
    pub(crate) max_packet_size: u32,
    pub(crate) data: T,
}
