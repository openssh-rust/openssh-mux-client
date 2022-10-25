use bytes::Bytes;
use serde::Deserialize;
use strum::IntoStaticStr;

use crate::{
    constants::*,
    error::{Error, OpenFailure},
};

fn deserialize<'a, T>(s: &'a [u8]) -> Result<T, Error>
where
    T: Deserialize<'a>,
{
    Ok(ssh_format::from_bytes(s)?.0)
}

mod channel;
pub(crate) use channel::*;

#[derive(Clone, Debug, IntoStaticStr)]
pub(crate) enum Response {
    GlobalRequestFailure,

    GlobalRequestSuccess,

    ChannelResponse {
        channel_response: ChannelResponse,
        recipient_channel: u32,
    },

    OpenChannelRequest,
}

impl Response {
    pub(crate) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        let (_padding_len, packet_type): (u8, u8) = deserialize(&bytes)?;

        let bytes = bytes.slice(2..);

        match packet_type {
            SSH_MSG_REQUEST_SUCCESS => Ok(Response::GlobalRequestSuccess),
            SSH_MSG_REQUEST_FAILURE => Ok(Response::GlobalRequestFailure),
            SSH_MSG_CHANNEL_OPEN => Ok(Response::OpenChannelRequest),
            packet_type => Ok(Response::ChannelResponse {
                recipient_channel: deserialize(&bytes)?,
                channel_response: ChannelResponse::from_packet(packet_type, bytes.slice(4..))?,
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ChannelResponse {
    OpenConfirmation(OpenConfirmation),
    OpenFailure(OpenFailure),

    BytesAdjust {
        bytes_to_add: u32,
    },
    Data(Bytes),
    ExtendedData {
        data_type: ExtendedDataType,
        data: Bytes,
    },
    Eof,
    Close,

    RequestSuccess,
    RequestFailure,

    Request(ChannelRequest),
}

impl ChannelResponse {
    fn from_packet(packet_type: u8, bytes: Bytes) -> Result<Self, Error> {
        use ChannelResponse::*;

        match packet_type {
            SSH_MSG_CHANNEL_OPEN_CONFIRMATION => Ok(OpenConfirmation(deserialize(&bytes)?)),
            SSH_MSG_CHANNEL_OPEN_FAILURE => Ok(OpenFailure(deserialize(&bytes)?)),
            SSH_MSG_CHANNEL_WINDOW_ADJUST => Ok(BytesAdjust {
                bytes_to_add: deserialize(&bytes)?,
            }),
            SSH_MSG_CHANNEL_DATA => Ok(Data(bytes)),
            SSH_MSG_CHANNEL_EXTENDED_DATA => {
                let (data_type, data) = ExtendedDataType::from_bytes(bytes)?;
                Ok(ExtendedData { data_type, data })
            }
            SSH_MSG_CHANNEL_EOF => Ok(Eof),
            SSH_MSG_CHANNEL_CLOSE => Ok(Close),

            SSH_MSG_CHANNEL_SUCCESS => Ok(RequestSuccess),
            SSH_MSG_CHANNEL_FAILURE => Ok(RequestFailure),

            SSH_MSG_CHANNEL_REQUEST => ChannelRequest::from_bytes(bytes).map(Request),

            _ => Err(Error::InvalidResponse(&"Unexpected packet type")),
        }
    }
}
