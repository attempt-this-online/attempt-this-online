mod codes;
use crate::codes::*;
use serde::{Deserialize, Serialize};
use std::io::Write;

macro_rules! log_error {
    ($($x:expr),*) => {
        // so user sees the error
        println!($($x,)*);
        // so ATO system log sees the error
        eprintln!($($x,)*);
    }
}

#[derive(Serialize)]
struct Response {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Request {
    language: String,
    code: Vec<u8>,
    input: Vec<u8>,
    arguments: Vec<Vec<u8>>,
    options: Vec<Vec<u8>>,
    timeout: i32,
}

fn main() -> std::process::ExitCode {
    let request = match rmp_serde::from_read::<_, Request>(std::io::stdin()) {
        Ok(r) => r,
        Err(e) => {
            log_error!("decode error: {}", e);
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    let _ = request; // TODO: use request
    let encoded_output = match rmp_serde::to_vec_named(&Response {
        stdout: b"hello".to_vec(),
        stderr: b"goodbye".to_vec(),
    }) {
        Ok(r) => r,
        Err(e) => {
            log_error!("error encoding output: {}", e);
            return std::process::ExitCode::from(INTERNAL_ERROR);
        }
    };
    match std::io::stdout().write_all(&encoded_output[..]) {
        Ok(()) => std::process::ExitCode::from(NORMAL),
        Err(e) => {
            log_error!("error writing output: {}", e);
            std::process::ExitCode::from(INTERNAL_ERROR)
        }
    }
}
