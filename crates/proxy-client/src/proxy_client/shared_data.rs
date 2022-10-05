use super::channel::MpscBytesChannel;

#[derive(Debug)]
pub(super) struct SharedData {
    pub(super) write_channel: MpscBytesChannel,
}
