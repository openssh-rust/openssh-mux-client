use std::convert::TryInto;

use bytes::{Bytes, BytesMut};
use serde::Serialize;
use ssh_format::Serializer;

use super::Error;

mod channel;
pub(crate) use channel::*;

mod port_forwarding;
pub(crate) use port_forwarding::*;

#[derive(Copy, Clone, Debug, Serialize)]
pub(crate) struct Request<T> {
    /// Must be 0
    padding_len: u8,
    packet_type: u8,
    packet: T,
}

impl<T: Serialize> Request<T> {
    fn new(packet_type: u8, packet: T) -> Self {
        Self {
            padding_len: 0,
            packet_type,
            packet,
        }
    }

    pub(crate) fn serialize_with_header(
        &self,
        bytes: &mut BytesMut,
        extra_data: usize,
    ) -> Result<Bytes, Error> {
        let extra_data: u32 = extra_data
            .try_into()
            .map_err(|_| ssh_format::Error::TooLong)?;

        // Reset bytes and reserve for the header
        bytes.resize(4, 0);

        // Serialize
        let mut serializer = Serializer::new(&mut *bytes);
        self.serialize(&mut serializer)?;

        // Write the header
        let header = serializer.create_header(extra_data)?;
        bytes[..4].copy_from_slice(&header);

        // Split and freeze it
        Ok(bytes.split().freeze())
    }
}
