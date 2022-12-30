#![allow(dead_code)]

#[allow(non_upper_case_globals)]
pub const KiB: u64 = 1024;
#[allow(non_upper_case_globals)]
pub const MiB: u64 = KiB * KiB;
pub const MAX_REQUEST_SIZE: usize = 64 * KiB as usize;
