#![allow(dead_code)]

pub const WEBSOCKET_BASE: u16 = 1000;
pub const NORMAL: u8 = 0;
pub const UNSUPPORTED_DATA: u8 = 3;
pub const POLICY_VIOLATION: u8 = 8;
pub const INTERNAL_ERROR: u8 = 11;
pub const MAX_REQUEST_SIZE: usize = 1 << 16; // 64KiB
