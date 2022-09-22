use bytes::Bytes;
use compact_str::CompactString;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenConfirmation {
    recipient_channel: u32,
    sender_channel: u32,
    init_win_size: u32,
    max_packet_size: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenFailure {
    recipient_channel: u32,
    reason_code: u32,
    description: CompactString,
    language_tag: CompactString,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct BytesAdjust {
    recipient_channel: u32,
    bytes_to_add: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelData {
    recipient_channel: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelExtendedData {
    recipient_channel: u32,
    data_type: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelEof {
    recipient_channel: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelClose {
    recipient_channel: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelRequestSuccess {
    recipient_channel: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelRequestFailure {
    recipient_channel: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelRequest {
    recipient_channel: u32,
    request_type: CompactString,
    want_reply: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelOpen {
    channel_type: CompactString,
    sender_channel: u32,
    init_win_size: u32,
    max_packet_size: u32,
}
