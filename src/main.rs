#![feature(
    io_error_more,
    let_chains,
    exitcode_exit_method,
    anonymous_lifetime_in_impl_trait,
    cursor_remaining,
)]

mod constants;
mod languages;
mod sandbox;
mod network;

use crate::{constants::*, languages::*, sandbox::invoke};
use nix::sys::signal::{signal, Signal, SigHandler};
use std::process::Termination;
use std::net::{SocketAddr, TcpListener, TcpStream};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::handshake::server as http;
use tungstenite::http::StatusCode;
use tungstenite as ws;
use serde::{Deserialize, de::DeserializeOwned, Serialize};
use serde_bytes::ByteBuf;

fn get_bind_address() -> SocketAddr {
    use std::str::FromStr;
    SocketAddr::from_str(
        &std::env::var("ATO_BIND").unwrap_or_else(|e| {
            if let std::env::VarError::NotUnicode(_) = e {
                panic!("$ATO_BIND is invalid Unicode")
            }
            "127.0.0.1:8500".to_string()
        })
    )
    .expect("$ATO_BIND is not a valid address")
}

/// analogous to std::thread::spawn but forks a full new process instead of a thread
fn fork_spawn<F, T>(f: F)
where F: FnOnce() -> T,
      F: 'static,
      T: Termination,
{
    use nix::unistd::{fork, ForkResult};
    // this is safe as long as the program only has one thread so far
    match unsafe { fork() }.unwrap() {
        ForkResult::Parent{..} => {},
        ForkResult::Child => {
            let termination = f();
            termination.report().exit_process();
        }
    }
}

fn main() {
    // tell the kernel not to keep zombie processes around
    // see waitpid(2) § NOTES and https://elixir.bootlin.com/linux/v6.1.2/source/kernel/signal.c#L2089
    // this is safe because there was no previous signal handler function
    unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) }.unwrap();

    let addr = get_bind_address();
    eprintln!("starting ATO server on {addr}");
    let server = TcpListener::bind(addr).unwrap();
    for connection in server.incoming() {
        fork_spawn(move || handle_ws(connection.unwrap()));
    }
}

fn handle_ws(connection: TcpStream) {
    // tell the kernel that we now *do* care about our child processes
    // see waitpid(2) § NOTES
    // this is safe because there was no previous signal handler function
    unsafe { signal(Signal::SIGCHLD, SigHandler::SigDfl) }.unwrap();

    // get raw fd so we can poll on it later
    use std::os::fd::AsRawFd;
    let connection_fd = connection.as_raw_fd();
    let mut config = WebSocketConfig::default();
    config.max_message_size = Some(MAX_REQUEST_SIZE);
    let websocket =
        match tungstenite::accept_hdr_with_config(connection, handle_headers, Some(config)) {
            Ok(ws) => ws,
            Err(tungstenite::HandshakeError::Failure(e)) => {
                match e {
                    _ => todo!("send 400 bad request error or something"),
                }
            }
            Err(e) => panic!("{}", e),
        };
    let mut connection = Connection(websocket);

    loop {
        use tungstenite::protocol::frame::{CloseFrame, coding::CloseCode};
        use std::borrow::Cow;

        fn close(
            connection: &mut Connection,
            code: CloseCode,
            reason: impl Into<Cow<'_, str>>,
        ) -> Result<(), ws::Error> {
            let frame = CloseFrame {
                code,
                reason: reason.into(),
            };
            connection.0.close(Some(frame))
        }

        let closed = match handle_request(&mut connection, connection_fd) {
            Ok(()) => continue, // don't close
            Err(Error::ClientWentAway) => connection.0.close(None),
            Err(Error::TooLarge(size)) =>
                close(&mut connection, CloseCode::Size, format!("received message of size {size}, greater than size limit {MAX_REQUEST_SIZE}")),
            Err(Error::UnsupportedData) => close(&mut connection, CloseCode::Unsupported, "expected a binary message"),
            Err(Error::PolicyViolation(e)) => close(&mut connection, CloseCode::Policy, format!("invalid request: {e}")),
            Err(Error::InternalError(e)) => {
                eprintln!("{e}");
                close(&mut connection, CloseCode::Error, e)
            },
        };
        match closed {
            Ok(()) | Err(ws::Error::ConnectionClosed) => (),
            Err(e) => {
                // can't do anything but log it
                eprintln!("error closing websocket: {e}")
            }
        }
        break
    }
    /* loop {
        match connection.0.write_pending() {
            Ok(()) => continue,
            Err(ws::Error::ConnectionClosed) => break,
            Err(e) => panic!("{e}"),
        }
    } */
}

fn handle_headers(request: &http::Request, response: http::Response) -> Result<http::Response, http::ErrorResponse> {
    if request.uri() != "/api/v1/ws/execute" {
        let response = http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Some("the only supported API URL is /api/v1/ws/execute".to_string()))
            .unwrap();
        Err(response)
    } else {
        Ok(response)
    }
}


#[derive(Serialize)]
enum StreamResponse {
    Stdout(ByteBuf),
    Stderr(ByteBuf),
    Done {
        timed_out: bool,
        stdout_truncated: bool,
        stderr_truncated: bool,
        status_type: &'static str,
        status_value: i32,
        real: i64,
        kernel: i64,
        user: i64,
        max_mem: i64,
        waits: i64,
        preemptions: i64,
        major_page_faults: i64,
        minor_page_faults: i64,
        input_ops: i64,
        output_ops: i64,
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Request {
    pub language: String,
    pub code: ByteBuf,
    pub input: ByteBuf,
    pub arguments: Vec<ByteBuf>,
    pub options: Vec<ByteBuf>,
    #[serde(default = "default_timeout")]
    pub timeout: i32,
}

fn default_timeout() -> i32 { 60 }

#[derive(Debug)]
pub enum Error {
    ClientWentAway,
    TooLarge(usize),
    UnsupportedData,
    PolicyViolation(String),
    InternalError(String),
}

/// like the ? postfix operator, but formats errors to strings
macro_rules! check {
    ($x:expr, $f:literal $(, $($a:expr),+)? $(,)?) => {
        $x.map_err(|e| Error::InternalError(format!($f, $($($a,)*)? e)))?
    };
    ($x:expr $(,)?) => {
        $x.map_err(|e| Error::InternalError(e.to_string()))?
    }
}

pub(crate) use check;

fn handle_request(connection: &mut Connection, connection_fd: i32) -> Result<(), Error> {
    let request = connection.read_message()?;
    let language = validate(&request)?;
    invoke(&request, language, connection, connection_fd)
}

fn validate(request: &Request) -> Result<&Language, Error> {
    if request.timeout < 1 || request.timeout > 60 {
        return Err(Error::PolicyViolation(format!("timeout not in range 1-60: {}", request.timeout)));
    }
    for arg in request.options.iter().chain(request.arguments.iter()) {
        if arg.contains(&0) {
            return Err(Error::PolicyViolation("argument contains null byte".to_string()));
        }
    }
    if let Some(l) = LANGUAGES.get(&request.language) {
        Ok(l)
    } else {
        Err(Error::PolicyViolation(format!("no such language: {}", &request.language)))
    }
}

#[derive(Debug, Deserialize)]
pub enum ControlMessage {
    Kill,
}

#[derive(Debug)]
pub struct Connection(ws::WebSocket<TcpStream>);

impl Connection {
    pub fn read_message<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let message = match self.0.read_message() {
            Ok(ws::Message::Binary(b)) => b,
            Ok(ws::Message::Close(_)) | Err(ws::Error::ConnectionClosed) =>
                return Err(Error::ClientWentAway),
            Ok(_) => return Err(Error::UnsupportedData),
            Err(ws::Error::Capacity(ws::error::CapacityError::MessageTooLong{size, ..})) =>
                return Err(Error::TooLarge(size)),
            Err(e) => {
                let e = format!("error reading request: {e}");
                return Err(Error::InternalError(e));
            }
        };
        let cursor = std::io::Cursor::new(&message);
        let mut de = rmp_serde::Deserializer::new(cursor);
        match <T as Deserialize>::deserialize(&mut de) {
            Ok(r) => {
                if !de.get_ref().is_empty() {
                    Err(Error::PolicyViolation("found extra data".to_string()))
                } else {
                    Ok(r)
                }
            },
            Err(e) => {
                Err(Error::PolicyViolation(e.to_string()))
            }
        }
    }

    pub fn output_message<T: Serialize>(&mut self, message: T) -> Result<(), Error> {
        let encoded_message = check!(rmp_serde::to_vec_named(&message), "error encoding output message: {}");
        match self.0.write_message(ws::Message::Binary(encoded_message)) {
            Ok(()) => Ok(()),
            Err(ws::Error::ConnectionClosed) => Err(Error::ClientWentAway),
            Err(e) => Err(Error::InternalError(format!("error writing output message: {e}"))),
        }
    }
}
