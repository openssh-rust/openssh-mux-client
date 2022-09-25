use std::num::NonZeroU32;

use bytes::Bytes;
use serde::Deserialize;

use crate::{constants::*, Error};

fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T, Error>
where
    T: Deserialize<'a>,
{
    Ok(ssh_format::from_bytes(s)?.0)
}

mod channel;
pub(crate) use channel::*;

#[derive(Clone, Debug)]
pub(crate) enum Response {
    GlobalRequestFailure,

    GlobalRequestSuccess {
        /// Response of global remote-forwarding request.
        port: Option<NonZeroU32>,
    },

    ChannelResponse {
        channel_response: ChannelResponse,
        recipient_channel: u32,
    },

    OpenChannelRequest {
        body: ChannelOpen,
        data: Bytes,
    },
}

impl Response {
    pub(crate) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        let (_padding_len, packet_type): (u8, u8) = from_bytes(&bytes)?;

        let bytes = bytes.slice(2..);

        match packet_type {
            SSH_MSG_REQUEST_SUCCESS => {
                let port = if bytes.len() == 4 {
                    Some(from_bytes(&bytes)?)
                } else {
                    None
                };
                Ok(Response::GlobalRequestSuccess { port })
            }
            SSH_MSG_REQUEST_FAILURE => Ok(Response::GlobalRequestFailure),
            SSH_MSG_CHANNEL_OPEN => {
                let (body, data) = ChannelOpen::from_bytes(bytes)?;
                Ok(Response::OpenChannelRequest { body, data })
            }
            packet_type => Ok(Response::ChannelResponse {
                recipient_channel: from_bytes(&bytes)?,
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
            SSH_MSG_CHANNEL_OPEN_CONFIRMATION => Ok(OpenConfirmation(from_bytes(&bytes)?)),
            SSH_MSG_CHANNEL_OPEN_FAILURE => Ok(OpenFailure(from_bytes(&bytes)?)),
            SSH_MSG_CHANNEL_WINDOW_ADJUST => Ok(BytesAdjust {
                bytes_to_add: from_bytes(&bytes)?,
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
