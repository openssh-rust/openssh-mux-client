use tokio::task::JoinHandle;

mod channel;

mod shared_data;
use shared_data::{ChannelDataArenaArc, SharedData};

mod read_task;
use read_task::create_read_task;

mod write_task;
use write_task::create_write_task;

pub struct ProxyClient {}
