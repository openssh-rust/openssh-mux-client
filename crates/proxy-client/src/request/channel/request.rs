use std::borrow::Cow;

use serde::Serialize;

use super::Request;
use crate::{constants::*, NonZeroByteSlice};

#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct ChannelRequest<T> {
    recipient_channel: u32,
    request_type: &'static &'static str,
    want_reply: bool,
    request_specific_data: T,
}

impl<T: Serialize> ChannelRequest<T> {
    fn new(
        recipient_channel: u32,
        request_type: &'static &'static str,
        request_specific_data: T,
    ) -> Request<ChannelRequest<T>> {
        Request::new(
            SSH_MSG_CHANNEL_REQUEST,
            Self {
                recipient_channel,
                request_type,
                want_reply: true,
                request_specific_data,
            },
        )
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PassEnv<'a> {
    name: Cow<'a, str>,
    value: Cow<'a, str>,
}

impl<'a> PassEnv<'a> {
    pub(crate) fn new(
        recipient_channel: u32,
        name: Cow<'a, str>,
        value: Cow<'a, str>,
    ) -> Request<ChannelRequest<Self>> {
        ChannelRequest::new(recipient_channel, &"env", Self { name, value })
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ExecCmd<'a>(Cow<'a, NonZeroByteSlice>);

impl<'a> ExecCmd<'a> {
    pub(crate) fn new(
        recipient_channel: u32,
        cmd: Cow<'a, NonZeroByteSlice>,
    ) -> Request<ChannelRequest<ExecCmd<'a>>> {
        ChannelRequest::new(recipient_channel, &"exec", Self(cmd))
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct RequestSubsystem(&'static &'static str);

impl RequestSubsystem {
    pub(crate) fn new(
        recipient_channel: u32,
        subsystem: &'static &'static str,
    ) -> Request<ChannelRequest<Self>> {
        ChannelRequest::new(recipient_channel, &"subsystem", Self(subsystem))
    }
}
