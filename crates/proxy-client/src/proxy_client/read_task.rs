use std::{
    collections::hash_map::Entry,
    convert::TryInto,
    num::NonZeroUsize,
    pin::Pin,
    sync::{atomic::Ordering::Relaxed, Arc},
};

use bytes::{Bytes, BytesMut};
use hash_hasher::HashedMap as HashMap;
use ssh_format::from_bytes;
use tokio::{io::AsyncRead, pin, spawn, task::JoinHandle};
use tokio_io_utility::read_to_bytes_rng;

use crate::{
    proxy_client::{
        channel::{
            Completion, MpscBytesChannel, OpenChannelRequestedInner, OpenChannelRes, ProcessStatus,
        },
        ChannelDataArenaArc, SharedData,
    },
    request::ChannelAdjustWindow,
    response::{ChannelRequest, ChannelResponse, ExtendedDataType, OpenConfirmation, Response},
    Error,
};

#[derive(Debug, Default)]
struct PendingRequests {
    pending: Option<NonZeroUsize>,
    /// Has any request failed
    has_failed: bool,
}

#[derive(Debug)]
struct ChannelIngoingData {
    outgoing_data_arena_arc: ChannelDataArenaArc,

    /// Once this get into zero and `outgoing_data.receivers_count != 0`,
    /// then read task should send `extend_window_size_packet`.
    receiver_win_size: u32,

    /// Check [`super::channel::ChannelState::extend_window_size`] for doc.
    extend_window_size: u32,

    pending_requests: PendingRequests,

    rx: Option<Arc<MpscBytesChannel>>,

    stderr: Option<Arc<MpscBytesChannel>>,
}

#[derive(Debug, Default)]
struct ChannelIngoingMap(HashMap<u32, ChannelIngoingData>);

impl ChannelIngoingMap {
    /// Insert a new entry with key `channel_id`.
    /// If such entry already exists, return an error.
    fn insert_new(&mut self, channel_id: u32, data: ChannelIngoingData) -> Result<(), Error> {
        if self.0.insert(channel_id, data).is_some() {
            Err(Error::DuplicateSenderChannel(channel_id))
        } else {
            Ok(())
        }
    }

    fn get(&mut self, channel_id: u32) -> Result<&mut ChannelIngoingData, Error> {
        self.0
            .get_mut(&channel_id)
            .ok_or(Error::InvalidSenderChannel(channel_id))
    }

    fn remove(&mut self, channel_id: u32) -> Result<ChannelIngoingData, Error> {
        self.0
            .remove(&channel_id)
            .ok_or(Error::InvalidSenderChannel(channel_id))
    }
}

/// If `is_rx` then `bytes` will be pushed to `rx`.
/// Otherwise it will be pushed to `stderr`.
fn handle_incoming_data(
    hashmap: &mut ChannelIngoingMap,
    recipient_channel: u32,
    bytes: Bytes,
    buffer: &mut BytesMut,
    shared_data: &SharedData,
    is_rx: bool,
) -> Result<(), Error> {
    let data = hashmap.get(recipient_channel)?;

    let cnt: u32 = bytes.len().try_into().unwrap_or(u32::MAX);

    let data_receiver_channel = if is_rx {
        data.rx.as_ref()
    } else {
        data.stderr.as_ref()
    };

    if let Some(channel) = data_receiver_channel {
        channel.push_bytes(bytes);
    }

    let receiver_win_size = &mut data.receiver_win_size;

    *receiver_win_size = receiver_win_size.saturating_sub(cnt);

    let outgoing_data = &data.outgoing_data_arena_arc;

    // Extend receiver window if it is 0 and there are still
    // active receivers
    if *receiver_win_size == 0 && outgoing_data.receivers_count.load(Relaxed) != 0 {
        let start = buffer.len();

        ChannelAdjustWindow::new(
            ChannelDataArenaArc::slot(outgoing_data),
            data.extend_window_size,
        )
        .serialize_with_header(buffer, 0)
        .unwrap();

        // After this op, buffer contains [0, start) which
        // contains the same content before extend_from_slice
        // and bytes contains `start..`
        let bytes = buffer.split_off(start).freeze();

        shared_data.get_write_channel().push_bytes(bytes);

        *receiver_win_size = data.extend_window_size;
    }

    Ok(())
}

fn mark_eof(data: &mut ChannelIngoingData) {
    if let Some(rx) = data.rx.take() {
        rx.mark_eof();
    }
    if let Some(stderr) = data.stderr.take() {
        stderr.mark_eof();
    }
}

fn handle_request_response(
    hashmap: &mut ChannelIngoingMap,
    recipient_channel: u32,
    success: bool,
) -> Result<(), Error> {
    let data = hashmap.get(recipient_channel)?;

    let pending = &mut data.pending_requests.pending;

    if pending.is_none() {
        // Retreive the latest information of pending requests

        *pending = data
            .outgoing_data_arena_arc
            .pending_requests
            .retrieve_pending_requests();

        // Reset has_failed
        data.pending_requests.has_failed = false;
    }

    *pending = NonZeroUsize::new(
        pending
            .as_mut()
            .ok_or(Error::UnexpectedRequestResponse)?
            .get()
            - 1,
    );

    data.pending_requests.has_failed |= !success;

    if pending.is_none() {
        // All pending requests are done

        let completion = if data.pending_requests.has_failed {
            Completion::Failed
        } else {
            Completion::Success
        };

        data.outgoing_data_arena_arc
            .pending_requests
            .report_request_completion(completion);
    }

    Ok(())
}

pub(super) fn create_read_task<R>(rx: R, shared_data: SharedData) -> JoinHandle<Result<(), Error>>
where
    R: AsyncRead + Send + 'static,
{
    spawn(async move {
        pin!(rx);

        create_read_task_inner(rx, shared_data).await
    })
}

async fn create_read_task_inner(
    mut rx: Pin<&mut (dyn AsyncRead + Send)>,
    shared_data: SharedData,
) -> Result<(), Error> {
    let mut buffer = BytesMut::with_capacity(1024);
    let mut ingoing_channel_map = ChannelIngoingMap::default();

    loop {
        read_and_handle_one_packet(
            rx.as_mut(),
            &shared_data,
            &mut buffer,
            &mut ingoing_channel_map,
        )
        .await?;
    }

    todo!()
}

async fn read_and_handle_one_packet(
    mut rx: Pin<&mut (dyn AsyncRead + Send)>,
    shared_data: &SharedData,
    buffer: &mut BytesMut,
    ingoing_channel_map: &mut ChannelIngoingMap,
) -> Result<(), Error> {
    read_to_bytes_rng(&mut rx, buffer, 4..).await?;

    let packet_len: u32 = from_bytes(&buffer[..4])?.0;
    let packet_len: usize = packet_len.try_into().unwrap();

    // Excluding the header (`u32`)
    let packet_bytes_read = buffer.len() - 4;

    if packet_bytes_read < packet_len {
        read_to_bytes_rng(&mut rx, buffer, (packet_len - packet_bytes_read)..).await?;
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
                let outgoing_data_arena_arc = shared_data.get_channel_data(recipient_channel)?;

                outgoing_data_arena_arc
                    .sender_window_size
                    .add(init_win_size.try_into().unwrap());

                let OpenChannelRequestedInner {
                    init_receiver_win_size,
                    extend_window_size,
                } = outgoing_data_arena_arc
                    .state
                    .set_channel_open_res(OpenChannelRes::Confirmed { max_packet_size })?;

                let ingoing_data = ChannelIngoingData {
                    rx: outgoing_data_arena_arc.rx.clone(),
                    stderr: outgoing_data_arena_arc.stderr.clone(),

                    outgoing_data_arena_arc,
                    receiver_win_size: init_receiver_win_size,
                    extend_window_size,

                    pending_requests: Default::default(),
                };

                ingoing_channel_map.insert_new(sender_channel, ingoing_data)?;
            }
            ChannelResponse::OpenFailure(failure) => {
                shared_data
                    .get_channel_data(recipient_channel)?
                    .state
                    .set_channel_open_res(OpenChannelRes::Failed(failure))?;
            }

            // Handle close of the channel
            ChannelResponse::Close => {
                let mut data = ingoing_channel_map.remove(recipient_channel)?;

                mark_eof(&mut data);
            }

            // Handle data related responses
            ChannelResponse::BytesAdjust { bytes_to_add } => ingoing_channel_map
                .get(recipient_channel)?
                .outgoing_data_arena_arc
                .sender_window_size
                .add(bytes_to_add.try_into().unwrap()),
            ChannelResponse::Data(bytes) => handle_incoming_data(
                ingoing_channel_map,
                recipient_channel,
                bytes,
                buffer,
                shared_data,
                true,
            )?,
            ChannelResponse::ExtendedData { data_type, data } => {
                if let ExtendedDataType::Stderr = data_type {
                    handle_incoming_data(
                        ingoing_channel_map,
                        recipient_channel,
                        data,
                        buffer,
                        shared_data,
                        false,
                    )?
                }
            }
            ChannelResponse::Eof => mark_eof(ingoing_channel_map.get(recipient_channel)?),

            // Handle responses to requests
            ChannelResponse::RequestSuccess => {
                handle_request_response(ingoing_channel_map, recipient_channel, true)?
            }
            ChannelResponse::RequestFailure => {
                handle_request_response(ingoing_channel_map, recipient_channel, false)?
            }

            // Handle incoming requests from sshd (exit status)
            ChannelResponse::Request(request) => {
                let process_status = match request {
                    ChannelRequest::StatusCode(exit_status) => {
                        ProcessStatus::ProcessExited(exit_status)
                    }
                    ChannelRequest::KilledBySignal(exit_signal) => {
                        ProcessStatus::ProcessKilled(exit_signal)
                    }
                    _ => {
                        return Err(Error::UnexpectedChannelState {
                            expected_state:
                                &"ChannelResponse::Request(StatusCode | KilledBySignal)",
                            actual_state: "ChannelResponse::Request(Unknown)",
                        })
                    }
                };

                ingoing_channel_map
                    .get(recipient_channel)?
                    .outgoing_data_arena_arc
                    .state
                    .set_channel_process_status(process_status)?;
            }
        }

        Ok(())
    } else {
        Err(Error::UnexpectedChannelState {
            expected_state: &"ChannelResponse",
            actual_state: response.into(),
        })
    }
}
