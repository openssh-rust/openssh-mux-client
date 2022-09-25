use std::borrow::Cow;

use bytes::Bytes;
use compact_str::CompactString;
use serde::Deserialize;

use crate::{
    response::{
        channel::{from_bytes_with_data, ExitSignal, ExitStatus},
        from_bytes,
    },
    Error,
};

#[derive(Deserialize)]
struct ChannelRequestHeader<'a> {
    #[serde(borrow)]
    pub(crate) request_type: Cow<'a, str>,
    pub(crate) want_reply: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum ChannelRequest {
    StatusCode(ExitStatus),
    KilledBySignal(ExitSignal),
    Unknown {
        request_type: CompactString,
        want_reply: bool,
        data: Bytes,
    },
}

impl ChannelRequest {
    pub(in crate::response) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        use ChannelRequest::*;

        let (header, data): (ChannelRequestHeader, _) = from_bytes_with_data(&bytes)?;
        Ok(match header.request_type.as_ref() {
            "exit-status" => StatusCode(from_bytes(&data)?),
            "exit-signal" => KilledBySignal(from_bytes(&data)?),
            _ => Unknown {
                request_type: header.request_type.into(),
                want_reply: header.want_reply,
                data,
            },
        })
    }
}
