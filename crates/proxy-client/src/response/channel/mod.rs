use compact_str::CompactString;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenConfirmation {
    sender_channel: u32,
    init_win_size: u32,
    max_packet_size: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenFailure {
    reason_code: u32,
    description: CompactString,
    language_tag: CompactString,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct BytesAdjust {
    bytes_to_add: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelExtendedData {
    data_type: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelRequest<T> {
    request_type: CompactString,
    want_reply: bool,
    data: T,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelOpen<T> {
    channel_type: CompactString,
    sender_channel: u32,
    init_win_size: u32,
    max_packet_size: u32,
    data: T,
}
