use std::convert::TryInto;

use serde::{Serialize, Serializer};

use super::{NonZeroByteSlice, Result};

/// Serialize one `u32` as ssh_format.
pub(crate) fn serialize_u32(int: u32) -> [u8; 4] {
    int.to_be_bytes()
}

pub(crate) enum MaybeOwned<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

impl<T> MaybeOwned<'_, T> {
    pub(crate) fn as_ref(&self) -> &T {
        use MaybeOwned::*;

        match self {
            Owned(val) => val,
            Borrowed(reference) => reference,
        }
    }
}

impl<T: Serialize> Serialize for MaybeOwned<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_ref().serialize(serializer)
    }
}

pub(crate) trait SliceExt {
    fn get_len_as_u32(&self) -> Result<u32>;
}

impl<T> SliceExt for [T] {
    fn get_len_as_u32(&self) -> Result<u32> {
        self.len()
            .try_into()
            .map_err(|_| ssh_format::Error::TooLong.into())
    }
}

impl SliceExt for str {
    fn get_len_as_u32(&self) -> Result<u32> {
        self.as_bytes().get_len_as_u32()
    }
}

impl SliceExt for NonZeroByteSlice {
    fn get_len_as_u32(&self) -> Result<u32> {
        self.into_inner().get_len_as_u32()
    }
}
