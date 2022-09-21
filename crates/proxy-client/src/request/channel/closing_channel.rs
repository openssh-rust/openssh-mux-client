use serde::Serialize;

use super::Request;
use crate::constants::*;

/// Signal that a party will no longer send more data to a channel.
#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct ChannelEof {
    recipient_channel: u32,
}

impl ChannelEof {
    pub(crate) fn new(recipient_channel: u32) -> Request<ChannelEof> {
        Request::new(SSH_MSG_CHANNEL_EOF, Self { recipient_channel })
    }
}

/// If wishes to terminate the channel, then sends this msg.
/// Upon receiving this message, a party MUST send back an
/// SSH_MSG_CHANNEL_CLOSE unless it has already sent this
/// message for the channel.  The channel is considered closed for a
/// party when it has both sent and received SSH_MSG_CHANNEL_CLOSE, and
/// the party may then reuse the channel number.  A party MAY send
/// SSH_MSG_CHANNEL_CLOSE without having sent or received
/// SSH_MSG_CHANNEL_EOF.
///
/// It is RECOMMENDED that all data sent before this message be
/// delivered to the actual destination, if possible.
#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct ChannelClose {
    recipient_channel: u32,
}

impl ChannelClose {
    pub(crate) fn new(recipient_channel: u32) -> Request<ChannelClose> {
        Request::new(SSH_MSG_CHANNEL_CLOSE, Self { recipient_channel })
    }
}
