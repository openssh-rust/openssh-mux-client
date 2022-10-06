use std::borrow::Cow;

use bytes::Bytes;
use serde::Deserialize;
use ssh_format::from_bytes;

use crate::{
    response::{
        channel::{ExitSignal, ExitStatus},
        deserialize,
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
    Unknown,
}

impl ChannelRequest {
    pub(in crate::response) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        use ChannelRequest::*;

        let (header, data): (ChannelRequestHeader, _) = from_bytes(&bytes)?;
        Ok(match header.request_type.as_ref() {
            "exit-status" => StatusCode(deserialize(data)?),
            "exit-signal" => KilledBySignal(deserialize(data)?),
            _ => Unknown,
        })
    }
}
