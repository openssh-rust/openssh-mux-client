use bytes::{Bytes, BytesMut};
use serde::Serialize;

use super::Request;
use crate::{constants::*, Error};

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

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DataTransfer {
    recipient_channel: u32,
    data_len: u32,
}

impl DataTransfer {
    fn new(recipient_channel: u32, data_len: u32) -> Request<Self> {
        Request::new(
            SSH_MSG_CHANNEL_WINDOW_ADJUST,
            Self {
                recipient_channel,
                data_len,
            },
        )
    }

    /// * `buffer` - This would not modify any existing data in it,
    ///   but it would create the header on it and split it out as a `Bytes`.
    pub(crate) fn create_header(
        recipient_channel: u32,
        data_len: u32,
        buffer: &mut BytesMut,
    ) -> Result<Bytes, Error> {
        let start = buffer.len();

        Self::new(recipient_channel, data_len).serialize_with_header(buffer, data_len)?;

        // After split_off, buffer contains [0, start), which is the
        // original content and the returned Bytes contains
        // [start, capacity), which is the header we just wrote.
        Ok(buffer.split_off(start).freeze())
    }
}
