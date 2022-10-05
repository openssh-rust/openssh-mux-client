use tokio::task::JoinHandle;

mod channel;

mod shared_data;
use shared_data::{ChannelDataArenaArc, SharedData};

pub struct ProxyClient {}
