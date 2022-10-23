use bytes::Bytes;
use compact_str::CompactString;
use serde::Deserialize;

use crate::Error;

mod exit_status;
pub(crate) use exit_status::*;

mod data;
pub(crate) use data::*;

mod request;
pub(crate) use request::*;

fn from_bytes_with_data<'de, T>(bytes: &'de Bytes) -> Result<(T, Bytes), Error>
where
    T: Deserialize<'de>,
{
    let (body, rest) = ssh_format::from_bytes(bytes)?;
    Ok((body, bytes.slice((bytes.len() - rest.len())..)))
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenConfirmation {
    pub(crate) sender_channel: u32,
    pub(crate) init_win_size: u32,
    pub(crate) max_packet_size: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelOpen {
    pub(crate) channel_type: CompactString,
    pub(crate) sender_channel: u32,
    pub(crate) init_win_size: u32,
    pub(crate) max_packet_size: u32,
}

impl ChannelOpen {
    pub(super) fn from_bytes(bytes: Bytes) -> Result<(Self, Bytes), Error> {
        from_bytes_with_data(&bytes)
    }
}
