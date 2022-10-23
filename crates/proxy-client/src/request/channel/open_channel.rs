use serde::Serialize;

use super::Request;
use crate::{constants::*, IpAddr};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OpenChannel<T> {
    channel_type: &'static &'static str,

    sender_channel: u32,
    initial_windows_size: u32,
    max_packet_size: u32,
    channel_specific_data: T,
}

impl<T: Serialize> OpenChannel<T> {
    fn new(
        channel_type: &'static &'static str,
        sender_channel: u32,
        initial_windows_size: u32,
        max_packet_size: u32,
        channel_specific_data: T,
    ) -> Request<OpenChannel<T>> {
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
    ) -> Request<OpenChannel<Session>> {
        OpenChannel::new(
            &"session",
            sender_channel,
            initial_windows_size,
            max_packet_size,
            Self(()),
        )
    }
}
