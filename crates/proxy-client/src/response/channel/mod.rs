use bytes::Bytes;
use compact_str::CompactString;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{Error, IpAddr};

pub(crate) mod error;
pub use error::*;

mod exit_status;
pub(crate) use exit_status::*;

mod data;
pub(crate) use data::*;

fn from_bytes<T>(bytes: Bytes) -> Result<(T, Bytes), Error>
where
    T: DeserializeOwned,
{
    let (body, rest) = ssh_format::from_bytes(&bytes)?;
    Ok((body, bytes.slice((bytes.len() - rest.len())..)))
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OpenConfirmation {
    pub(crate) sender_channel: u32,
    pub(crate) init_win_size: u32,
    pub(crate) max_packet_size: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChannelRequest {
    pub(crate) request_type: CompactString,
    pub(crate) want_reply: bool,
}

impl ChannelRequest {
    pub(super) fn from_bytes(bytes: Bytes) -> Result<(Self, Bytes), Error> {
        from_bytes(bytes)
    }
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
        from_bytes(bytes)
    }
}

/// This is to be used with `ChannelOpen`.
/// remote port forwarding
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ForwardedTcpIp<'a> {
    /// The socket that is remote forwarded
    #[serde(borrow)]
    pub(crate) connected_addr: IpAddr<'a>,
    /// The socket that connects to the remote forwarded
    /// connected_addr.
    #[serde(borrow)]
    pub(crate) originator_addr: IpAddr<'a>,
}
