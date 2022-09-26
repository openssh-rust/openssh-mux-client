use std::borrow::Cow;

use compact_str::CompactString;
use serde::{de::Deserializer, Deserialize};

use super::ErrMsg;

#[derive(Copy, Clone, Debug, Deserialize)]
#[repr(transparent)]
pub(crate) struct ExitStatus(pub u32);

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ExitSignal {
    pub signal_name: SignalName,
    pub core_dumped: bool,
    pub err_msg: ErrMsg,
}

#[derive(Clone, Debug)]
pub(crate) enum SignalName {
    Abrt,
    Alrm,
    Fpe,
    Hup,
    Ill,
    Int,
    Kill,
    Pipe,
    Quit,
    Segv,
    Term,
    Usr1,
    Usr2,

    /// Additional 'signal name' values MAY be sent in the format
    /// "sig-name@xyz", where "sig-name" and "xyz" may be anything a
    /// particular implementer wants (except the "@" sign).
    ///
    /// However, it is suggested that if a 'configure' script is used,
    /// any non-standard 'signal name' values it finds be encoded as
    /// "SIG@xyz.config.guess", where "SIG" is the 'signal name' without
    /// the "SIG" prefix, and "xyz" is the host type, as determined
    /// by "config.guess".
    Extension(CompactString),
}
impl<'de> Deserialize<'de> for SignalName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use SignalName::*;

        let signal = <Cow<'de, str> as Deserialize>::deserialize(deserializer)?;

        Ok(match signal.as_ref() {
            "ABRT" => Abrt,
            "ALRM" => Alrm,
            "FPE" => Fpe,
            "HUP" => Hup,
            "ILL" => Ill,
            "INT" => Int,
            "KILL" => Kill,
            "PIPE" => Pipe,
            "QUIT" => Quit,
            "SEGV" => Segv,
            "TERM" => Term,
            "USR1" => Usr1,
            "USR2" => Usr2,
            _ => Extension(signal.into()),
        })
    }
}
