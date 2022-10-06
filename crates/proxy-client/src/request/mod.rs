use bytes::BytesMut;
use serde::Serialize;
use ssh_format::{SerOutput, Serializer};

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

    fn serialize_with_header_inner(
        &self,
        bytes: &mut BytesMut,
        extra_data: u32,
    ) -> Result<(), Error> {
        // Reserve for the header
        let start = bytes.len();
        bytes.extend_from_slice(&[0_u8, 0_u8, 0_u8, 0_u8]);

        // Serialize
        let mut serializer = Serializer::new(&mut *bytes);
        self.serialize(&mut serializer)?;

        // Write the header
        let header = serializer.create_header(extra_data)?;
        bytes[start..(start + 4)].copy_from_slice(&header);

        Ok(())
    }

    /// On error, `bytes` stays unchanged.
    pub(crate) fn serialize_with_header(
        &self,
        bytes: &mut BytesMut,
        extra_data: u32,
    ) -> Result<(), Error> {
        let start = bytes.len();

        let res = self.serialize_with_header_inner(bytes, extra_data);

        if res.is_err() {
            bytes.truncate(start);
        }

        res
    }

    /// If the slice is not large enough, the function will panic.
    ///
    /// Return number of bytes written.
    pub(crate) fn serialize_to_slice(
        &self,
        slice: &mut [u8],
        extra_data: u32,
    ) -> Result<usize, Error> {
        // Serialize
        let mut buffer = SliceOutput(&mut slice[4..], 0);

        let mut serializer = Serializer::new(&mut buffer);
        self.serialize(&mut serializer)?;

        // Write the header
        let header = serializer.create_header(extra_data)?;

        let cnt = buffer.1;

        slice[..4].copy_from_slice(&header);

        // Split and freeze it
        Ok(4 + cnt)
    }
}

#[derive(Debug)]
struct SliceOutput<'a>(&'a mut [u8], usize);

impl SerOutput for SliceOutput<'_> {
    fn extend_from_slice(&mut self, other: &[u8]) {
        let start = self.1;
        let end = start + other.len();

        self.0[start..end].copy_from_slice(other);

        self.1 = end;
    }

    fn push(&mut self, byte: u8) {
        self.0[self.1] = byte;

        self.1 += 1;
    }

    fn reserve(&mut self, additional: usize) {
        if additional > self.0.len() {
            panic!("The slice is not large enough!")
        }
    }
}
