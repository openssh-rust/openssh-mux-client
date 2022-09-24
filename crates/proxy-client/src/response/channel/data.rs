use std::fmt;

use bytes::Bytes;
use serde::{de::Deserializer, Deserialize};

use crate::{constants::*, response::from_bytes, Error};

#[derive(Copy, Clone, Debug)]
pub(crate) enum ExtendedDataType {
    Stderr,
    Unknown,
}
impl<'de> Deserialize<'de> for ExtendedDataType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use ExtendedDataType::*;

        let code = <u32 as Deserialize>::deserialize(deserializer)?;

        Ok(match code {
            SSH_EXTENDED_DATA_STDERR => Stderr,
            _ => Unknown,
        })
    }
}
impl fmt::Display for ExtendedDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl ExtendedDataType {
    pub(in crate::response) fn from_bytes(bytes: Bytes) -> Result<(Self, Bytes), Error> {
        Ok((from_bytes(&bytes)?, bytes.slice(4..)))
    }
}
