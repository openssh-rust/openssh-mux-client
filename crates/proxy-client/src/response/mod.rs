use bytes::Bytes;
use ssh_format::from_bytes;

use crate::{constants::*, Error};

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

    OpenChannelRequest(Bytes),
}

impl Response {
    pub(crate) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        let (_padding_len, packet_type): (u8, u8) = from_bytes(&bytes)?.0;

        let bytes = bytes.slice(2..);

        match packet_type {
            SSH_MSG_REQUEST_SUCCESS => Ok(Response::GlobalRequestSuccess(bytes)),
            SSH_MSG_REQUEST_FAILURE => Ok(Response::GlobalRequestFailure),
            SSH_MSG_CHANNEL_OPEN => Ok(Response::OpenChannelRequest(bytes)),
            packet_type => Ok(Response::ChannelResponse {
                channel_response_type: ChannelResponseType::from_packet_type(packet_type)?,
                recipient_channel: from_bytes(&bytes)?.0,
                data: bytes.slice(4..),
            }),
        }
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

impl ChannelResponseType {
    fn from_packet_type(packet_type: u8) -> Result<Self, Error> {
        use ChannelResponseType::*;

        match packet_type {
            SSH_MSG_CHANNEL_OPEN_CONFIRMATION => Ok(OpenConfirmation),
            SSH_MSG_CHANNEL_OPEN_FAILURE => Ok(OpenFailure),
            SSH_MSG_CHANNEL_WINDOW_ADJUST => Ok(BytesAdjust),
            SSH_MSG_CHANNEL_DATA => Ok(ChannelData),
            SSH_MSG_CHANNEL_EXTENDED_DATA => Ok(ChannelExtendedData),
            SSH_MSG_CHANNEL_EOF => Ok(ChannelEof),
            SSH_MSG_CHANNEL_CLOSE => Ok(ChannelClose),
            SSH_MSG_CHANNEL_SUCCESS => Ok(ChannelRequestSuccess),
            SSH_MSG_CHANNEL_FAILURE => Ok(ChannelRequestFailure),
            SSH_MSG_CHANNEL_REQUEST => Ok(ChannelRequest),
            _ => Err(Error::InvalidResponse(&"Unexpected packet type")),
        }
    }
}
