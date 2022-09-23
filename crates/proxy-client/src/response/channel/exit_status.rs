use std::borrow::Cow;

use compact_str::CompactString;
use serde::{de::Deserializer, Deserialize};

use super::ErrMsg;

#[derive(Copy, Clone, Debug, Deserialize)]
pub(crate) struct ExitStatus(u32);

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ExitSignal {
    /// signal name (without the "SIG" prefix)
    pub signal_name: CompactString,
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

        Ok(match signal {
            Cow::Borrowed("ABRT") => Abrt,
            Cow::Borrowed("ALRM") => Alrm,
            Cow::Borrowed("FPE") => Fpe,
            Cow::Borrowed("HUP") => Hup,
            Cow::Borrowed("ILL") => Ill,
            Cow::Borrowed("INT") => Int,
            Cow::Borrowed("KILL") => Kill,
            Cow::Borrowed("PIPE") => Pipe,
            Cow::Borrowed("QUIT") => Quit,
            Cow::Borrowed("SEGV") => Segv,
            Cow::Borrowed("TERM") => Term,
            Cow::Borrowed("USR1") => Usr1,
            Cow::Borrowed("USR2") => Usr2,
            signal => Extension(signal.into()),
        })
    }
}
