use bytes::Bytes;
use ssh_format::{Deserializer, Error};

mod channel;
pub(crate) use channel::*;

#[derive(Clone, Debug)]
pub(crate) enum Response {
    GlobalRequestFailure,

    GlobalRequestSuccess(
        /// Request specific data
        Bytes,
    ),

    ChannelResponse {
        channel_response_type: ChannelResponseType,
        recipient_channel: u32,
        data: Bytes,
    },

    OpenChannelRequest {
        header: ChannelOpen,
        data: Bytes,
    },
}

impl Response {
    pub(crate) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ChannelResponseType {
    OpenConfirmation,
    OpenFailure,
    BytesAdjust,
    ChannelData,
    ChannelExtendedData,
    ChannelEof,
    ChannelClose,
    ChannelRequestSuccess,
    ChannelRequestFailure,
    ChannelRequest,
}
