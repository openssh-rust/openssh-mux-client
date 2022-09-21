use serde::Serialize;

use super::Request;
use crate::{constants::*, IpAddr};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Channel<T> {
    #[serde(borrow)]
    channel_type: &'static &'static str,

    sender_channel: u32,
    initial_windows_size: u32,
    max_packet_size: u32,
    channel_specific_data: T,
}

impl<T: Serialize> Channel<T> {
    fn new(
        channel_type: &'static &'static str,
        sender_channel: u32,
        initial_windows_size: u32,
        max_packet_size: u32,
        channel_specific_data: T,
    ) -> Request<Channel<T>> {
        Request::new(
            SSH_MSG_CHANNEL_OPEN,
            Self {
                channel_type,
                sender_channel,
                initial_windows_size,
                max_packet_size,
                channel_specific_data,
            },
        )
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct Session(());

impl Session {
    pub(crate) fn new(
        sender_channel: u32,
        initial_windows_size: u32,
        max_packet_size: u32,
    ) -> Request<Channel<Session>> {
        Channel::new(
            &"session",
            sender_channel,
            initial_windows_size,
            max_packet_size,
            Self(()),
        )
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LocalForward<'a> {
    /// The remote addr to connect to
    remote_addr: IpAddr<'a>,
    /// The local addr where the connection request
    /// originates from.
    originator_addr: IpAddr<'a>,
}

impl<'a> LocalForward<'a> {
    pub(crate) fn new(
        sender_channel: u32,
        initial_windows_size: u32,
        max_packet_size: u32,
        remote_addr: IpAddr<'a>,
        originator_addr: IpAddr<'a>,
    ) -> Request<Channel<LocalForward<'a>>> {
        Channel::new(
            &"direct-tcpip",
            sender_channel,
            initial_windows_size,
            max_packet_size,
            Self {
                remote_addr,
                originator_addr,
            },
        )
    }
}
