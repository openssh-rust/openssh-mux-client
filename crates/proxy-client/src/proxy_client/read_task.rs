use std::{
    collections::hash_map::Entry, convert::TryInto, num::NonZeroUsize, pin::Pin,
    sync::atomic::Ordering::Relaxed,
};

use bytes::BytesMut;
use hash_hasher::HashedMap as HashMap;
use ssh_format::from_bytes;
use tokio::{io::AsyncRead, pin, spawn, task::JoinHandle};
use tokio_io_utility::read_to_bytes_rng;

use crate::{
    proxy_client::{
        channel::{OpenChannelRequestedInner, OpenChannelRes},
        ChannelDataArenaArc, SharedData,
    },
    response::{ChannelResponse, OpenConfirmation, Response},
    Error,
};

#[derive(Debug)]
enum PendingRequests {
    Pending {
        pending: NonZeroUsize,
        /// Has any request failed
        has_failed: bool,
    },
    Done,
}

#[derive(Debug)]
struct ChannelIngoingData {
    outgoing_data_arena_arc: ChannelDataArenaArc,

    /// Once this get into zero and `outgoing_data.receivers_count != 0`,
    /// then read task should send `extend_window_size_packet`.
    receiver_win_size: u32,

    /// Check [`super::channel::ChannelState::extend_window_size_packet`] for doc.
    extend_window_size_packet: [u8; 14],

    /// Check [`super::channel::ChannelState::extend_window_size`] for doc.
    extend_window_size: u32,

    pending_requests: PendingRequests,
}

fn get_ingoing_data(
    hashmap: &mut HashMap<u32, ChannelIngoingData>,
    channel_id: u32,
) -> Result<&mut ChannelIngoingData, Error> {
    hashmap
        .get_mut(&channel_id)
        .ok_or(Error::InvalidSenderChannel(channel_id))
}

pub(super) fn create_read_task<R>(rx: R, shared_data: SharedData) -> JoinHandle<Result<(), Error>>
where
    R: AsyncRead + Send + 'static,
{
    async fn inner(
        mut rx: Pin<&mut (dyn AsyncRead + Send)>,
        shared_data: SharedData,
    ) -> Result<(), Error> {
        let mut buffer = BytesMut::with_capacity(1024);
        let mut ingoing_channel_map: HashMap<u32, ChannelIngoingData> = HashMap::default();

        read_to_bytes_rng(&mut rx, &mut buffer, 4..).await?;

        let packet_len: u32 = from_bytes(&buffer[..4])?.0;
        let packet_len: usize = packet_len.try_into().unwrap();

        // Excluding the header (`u32`)
        let packet_bytes_read = buffer.len() - 4;

        if packet_bytes_read < packet_len {
            read_to_bytes_rng(&mut rx, &mut buffer, (packet_len - packet_bytes_read)..).await?;
        }

        // Split until (packet_len + 4).
        // Afterwards, buffer would contain `(packet_len + 4)..`,
        // and the returned bytes contains``..(packet_len + 4)`.
        let response = Response::from_bytes(buffer.split_to(packet_len + 4).freeze().slice(4..))?;

        if let Response::ChannelResponse {
            channel_response,
            recipient_channel,
        } = response
        {
            match channel_response {
                // Handle response to open channel request
                ChannelResponse::OpenConfirmation(OpenConfirmation {
                    sender_channel,
                    init_win_size,
                    max_packet_size,
                }) => {
                    let outgoing_data_arena_arc =
                        shared_data.get_channel_data(recipient_channel)?;

                    outgoing_data_arena_arc
                        .sender_window_size
                        .add(init_win_size.try_into().unwrap());

                    let OpenChannelRequestedInner {
                        init_receiver_win_size,
                        extend_window_size_packet,
                        extend_window_size,
                    } = outgoing_data_arena_arc
                        .state
                        .set_channel_open_res(OpenChannelRes::Confirmed { max_packet_size })?;

                    let ingoing_data = ChannelIngoingData {
                        outgoing_data_arena_arc,
                        receiver_win_size: init_receiver_win_size,
                        extend_window_size_packet,
                        extend_window_size,
                        pending_requests: PendingRequests::Done,
                    };

                    match ingoing_channel_map.entry(sender_channel) {
                        Entry::Occupied(_) => {
                            return Err(Error::DuplicateSenderChannel(sender_channel));
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(ingoing_data);
                        }
                    }
                }
                ChannelResponse::OpenFailure(failure) => {
                    shared_data
                        .get_channel_data(recipient_channel)?
                        .state
                        .set_channel_open_res(OpenChannelRes::Failed(failure))?;
                }

                // Handle data related responses
                ChannelResponse::BytesAdjust { bytes_to_add } => {
                    get_ingoing_data(&mut ingoing_channel_map, recipient_channel)?
                        .outgoing_data_arena_arc
                        .sender_window_size
                        .add(bytes_to_add.try_into().unwrap())
                }
                ChannelResponse::Data(bytes) => {
                    let data = get_ingoing_data(&mut ingoing_channel_map, recipient_channel)?;

                    let cnt: u32 = bytes.len().try_into().unwrap_or(u32::MAX);

                    let outgoing_data = &*data.outgoing_data_arena_arc;

                    if let Some(rx) = outgoing_data.rx.as_ref() {
                        rx.push_bytes(bytes);
                    }

                    let receiver_win_size = &mut data.receiver_win_size;

                    *receiver_win_size = receiver_win_size.saturating_sub(cnt);

                    // Extend receiver window if it is 0 and there are still
                    // active receivers
                    if *receiver_win_size == 0 && outgoing_data.receivers_count.load(Relaxed) != 0 {
                        let start = buffer.len();
                        buffer.extend_from_slice(&data.extend_window_size_packet);

                        // After this op, buffer contains [0, start) which
                        // contains the same content before extend_from_slice
                        // and bytes contains `start..`
                        let bytes = buffer.split_off(start).freeze();

                        shared_data.get_write_channel().push_bytes(bytes);

                        *receiver_win_size = data.extend_window_size;
                    }
                }
                _ => todo!(),
            }
        } else {
            return Err(Error::UnexpectedChannelState {
                expected_state: &"ChannelResponse",
                actual_state: response.into(),
            });
        }

        todo!()
    }

    spawn(async move {
        pin!(rx);

        inner(rx, shared_data).await
    })
}
