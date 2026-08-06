use std::time::Duration;

pub const VID: u16 = 0x0483;
pub const PID: u16 = 0xF123;
pub const INTERFACE: u8 = 0;
pub const EP_OUT: u8 = 0x01;
pub const EP_IN: u8 = 0x81;
pub const READ_BUF: usize = 256;
pub const TIMEOUT: Duration = Duration::from_millis(3000);
pub const DRAIN: Duration = Duration::from_millis(100);
pub const EMPTY_READ_RETRIES: usize = 3;
