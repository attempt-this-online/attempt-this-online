mod codes;
mod languages;
use crate::{codes::*, languages::*};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
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
    stdout: ByteBuf,
    stderr: ByteBuf,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Request {
    language: String,
    code: ByteBuf,
    input: ByteBuf,
    arguments: Vec<ByteBuf>,
    options: Vec<ByteBuf>,
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
    let language = match validate(&request) {
        Ok(l) => l,
        Err(e) => {
            log_error!("invalid request: {}", e);
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    let result = match invoke(&request, language) {
        Ok(r) => r,
        Err(_) => {
            return std::process::ExitCode::from(INTERNAL_ERROR);
        }
    };
    let encoded_output = match rmp_serde::to_vec_named(&result) {
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

enum ValidationError<'a> {
    NoSuchLanguage(&'a String),
    NullByteInArgument,
}

impl<'a> std::fmt::Display for ValidationError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::NoSuchLanguage(name) => write!(f, "no such language: {}", name),
            ValidationError::NullByteInArgument => write!(f, "argument contains null byte"),
        }
    }
}

fn validate(request: &Request) -> Result<&Language, ValidationError> {
    for arg in request.options.iter().chain(request.arguments.iter()) {
        if arg.contains(&0) {
            return Err(ValidationError::NullByteInArgument);
        }
    }
    if let Some(l) = LANGUAGES.get(&request.language) {
        Ok(l)
    } else {
        Err(ValidationError::NoSuchLanguage(&request.language))
    }
}

fn invoke(request: &Request, language: &Language) -> Result<Response, std::convert::Infallible> {
    let _ = (request, language);
    Ok(Response {
        stdout: ByteBuf::from(b"hello".to_vec()),
        stderr: ByteBuf::from(b"goodbye".to_vec()),
    })
}
