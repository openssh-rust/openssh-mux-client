use core::marker::PhantomData;
use serde::{
    de::{Deserializer, EnumAccess, Error, VariantAccess, Visitor},
    Deserialize,
};
use std::fmt;

use super::constants;

/// **WARNING: Response can only be used with ssh_mux_format, which treats
/// tuple and struct as the same.**
#[derive(Clone, Debug)]
pub enum Response {
    Hello { version: u32 },

    Alive { response_id: u32, server_pid: u32 },

    Ok { response_id: u32 },
    Failure { response_id: u32, reason: String },

    PermissionDenied { response_id: u32, reason: String },

    SessionOpened { response_id: u32, session_id: u32 },
    ExitMessage { session_id: u32, exit_value: u32 },
    TtyAllocFail { session_id: u32 },

    RemotePort { response_id: u32, remote_port: u32 },
}
impl<'de> Deserialize<'de> for Response {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_enum(
            "Response",
            &[
                "Hello",
                "Alive",
                "Ok",
                "Failure",
                "PermissionDenied",
                "SessionOpened",
                "ExitMessage",
                "TtyAllocFail",
                "RemotePort",
            ],
            ResponseVisitor,
        )
    }
}

pub struct ResponseVisitor;
impl<'de> Visitor<'de> for ResponseVisitor {
    type Value = Response;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "expecting Response")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        use constants::*;

        let result: (u32, _) = data.variant()?;
        let (index, accessor) = result;

        match index {
            MUX_MSG_HELLO => {
                let version: u32 = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::Hello { version })
            }
            MUX_S_ALIVE => {
                let tup: (u32, u32) = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::Alive {
                    response_id: tup.0,
                    server_pid: tup.1,
                })
            }
            MUX_S_OK => {
                let response_id: u32 = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::Ok { response_id })
            }
            MUX_S_FAILURE => {
                let tup: (u32, String) = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::Failure {
                    response_id: tup.0,
                    reason: tup.1,
                })
            }
            MUX_S_PERMISSION_DENIED => {
                let tup: (u32, String) = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::PermissionDenied {
                    response_id: tup.0,
                    reason: tup.1,
                })
            }
            MUX_S_SESSION_OPENED => {
                let tup: (u32, u32) = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::SessionOpened {
                    response_id: tup.0,
                    session_id: tup.1,
                })
            }
            MUX_S_EXIT_MESSAGE => {
                let tup: (u32, u32) = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::ExitMessage {
                    session_id: tup.0,
                    exit_value: tup.1,
                })
            }
            MUX_S_TTY_ALLOC_FAIL => {
                let session_id: u32 = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::TtyAllocFail { session_id })
            }
            MUX_S_REMOTE_PORT => {
                let tup: (u32, u32) = accessor.newtype_variant_seed(PhantomData)?;
                Ok(Response::RemotePort {
                    response_id: tup.0,
                    remote_port: tup.1,
                })
            }
            _ => Err(A::Error::custom("Unexpected packet type")),
        }
    }
}
