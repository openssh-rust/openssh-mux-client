use std::fmt;

use serde::{de::Deserializer, Deserialize};
use thiserror::Error as ThisError;
use vec_strings::TwoStrs;

use crate::constants::*;

#[derive(Clone, Deserialize, Debug)]
#[repr(transparent)]
pub struct ErrMsg(TwoStrs);

impl ErrMsg {
    /// Returns (err_message, language_tag).
    ///
    /// Language tag is defined according to specification [RFC-1766].
    ///
    /// It can be parsed by
    /// [pyfisch/rust-language-tags](https://github.com/pyfisch/rust-language-tags)
    /// according to
    /// [this issue](https://github.com/pyfisch/rust-language-tags/issues/39).
    pub fn get(&self) -> (&str, &str) {
        self.0.get()
    }
}

impl fmt::Display for ErrMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (err_msg, language_tag) = self.get();
        write!(
            f,
            "Err Message: {}, Language Tag: {}",
            err_msg, language_tag
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ErrorCode {
    AdministrativelyProhibited,
    ConnectFailed,
    UnknownChannelType,
    ResourceShortage,
    Unknown,
}
impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use ErrorCode::*;

        let code = <u32 as Deserialize>::deserialize(deserializer)?;

        Ok(match code {
            SSH_OPEN_ADMINISTRATIVELY_PROHIBITED => AdministrativelyProhibited,
            SSH_OPEN_CONNECT_FAILED => ConnectFailed,
            SSH_OPEN_UNKNOWN_CHANNEL_TYPE => UnknownChannelType,
            SSH_OPEN_RESOURCE_SHORTAGE => ResourceShortage,
            _ => Unknown,
        })
    }
}
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

#[derive(Clone, Debug, Deserialize, ThisError)]
#[error("Failed to open new channel: code = {error_code}, msg = {err_msg}")]
pub struct OpenFailure {
    pub error_code: ErrorCode,
    pub err_msg: ErrMsg,
}
