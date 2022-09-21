use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpAddr<'a> {
    #[serde(borrow)]
    host: Cow<'a, str>,
    port: u32,
}

impl IpAddr<'_> {
    pub fn into_owned(self) -> IpAddr<'static> {
        IpAddr {
            host: Cow::Owned(self.host.into_owned()),
            port: self.port,
        }
    }
}
