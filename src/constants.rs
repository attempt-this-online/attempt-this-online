#![allow(dead_code)]

pub const WEBSOCKET_BASE: u16 = 1000;
pub const NORMAL: u8 = 0;
pub const UNSUPPORTED_DATA: u8 = 3;
pub const POLICY_VIOLATION: u8 = 8;
pub const INTERNAL_ERROR: u8 = 11;
pub const MAX_REQUEST_SIZE: usize = 1 << 16; // 64KiB
#[allow(non_upper_case_globals)]
pub const KiB: u64 = 1024;
#[allow(non_upper_case_globals)]
pub const MiB: u64 = KiB * KiB;

pub const PIPE_BUF: usize = 4096; // from limits.h
pub const OUTPUT_BUF_SIZE: usize = PIPE_BUF - 256; // subtract enough to account for msgpack overhead
