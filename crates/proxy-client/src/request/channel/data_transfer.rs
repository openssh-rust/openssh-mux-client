use serde::Serialize;

use super::Request;
use crate::constants::*;

#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct ChannelAdjustWindow {
    recipient_channel: u32,
    bytes_to_add: u32,
}

impl ChannelAdjustWindow {
    pub(crate) fn new(recipient_channel: u32, bytes_to_add: u32) -> Request<ChannelAdjustWindow> {
        Request::new(
            SSH_MSG_CHANNEL_WINDOW_ADJUST,
            Self {
                recipient_channel,
                bytes_to_add,
            },
        )
    }
}
