#![feature(let_chains)]
extern crate lazy_static;

mod constants;
use crate::constants::*;
use futures_util::{SinkExt, StreamExt};
use std::error::Error;
use std::net::SocketAddr;
use std::process::Stdio;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};
use tokio::task;
use tokio_tungstenite::tungstenite::error as ws_error;
use warp::ws::*;
use warp::Filter;

#[tokio::main]
async fn main() {
    let execute = warp::path!("api" / "v1" / "ws" / "execute")
        .and(warp::ws())
        .map(|ws: Ws| ws.max_message_size(MAX_REQUEST_SIZE).on_upgrade(handle_ws))
        .with(warp::cors().allow_any_origin());
    let addr = get_bind_address();
    eprintln!("starting ATO server on {addr}");
    warp::serve(execute).run(addr).await;
}

fn get_bind_address() -> SocketAddr {
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

async fn handle_ws(mut websocket: WebSocket) {
    loop {
        match invoke(&mut websocket).await {
            Ok(()) => continue,
            Err(None) => break,
            Err(Some((code, msg))) => {
                eprintln!("{msg}");
                if let Err(e) = websocket.send(Message::close_with(WEBSOCKET_BASE + code as u16, msg)).await {
                    if let Some(e) = e.source() && let Some(ws_error::Error::ConnectionClosed) = e.downcast_ref() {
                        // client went away: do nothing
                    } else {
                        // can't do anything but log it
                        eprintln!("error closing websocket: {e}");
                    }
                }
                return;
            }
        }
    }
    if let Err(e) = websocket.close().await {
        // can't do anything but log it
        eprintln!("error closing websocket: {e}");
    }
}

async fn invoke(
    websocket: &mut WebSocket,
) -> Result<(), Option<(u8, String)>> {
    let (mut stdin, mut child) =
        match invoke1().await {
            Ok(c) => c,
            Err(e) => {
                return Err(Some((INTERNAL_ERROR, e)))
            }
        };
    let mut stdout = child.stdout.take().expect("stdout should not have been taken");
    let mut wait = task::spawn(async { invoke2(child).await });
    let mut buf = [0u8; PIPE_BUF];
    loop {
        tokio::select! {
            res = &mut wait => return res.unwrap().map_err(Some),
            maybe_msg = websocket.next() => match maybe_msg {
                None => return wait.await.unwrap().map_err(Some),
                Some(msg) => handle_message(msg, &mut stdin).await?,
            },
            maybe_n = stdout.read(&mut buf) => match maybe_n {
                Err(e) => eprintln!("error reading message: {e}"),
                Ok(0) => { // EOF
                    return wait.await.unwrap().map_err(Some)
                }
                Ok(n) => {
                    if let Err(e) = websocket.send(Message::binary(&buf[..n])).await {
                        eprintln!("error sending to websocket: {e}")
                    }
                }
            }
        }
    }
}

async fn handle_message(msg: Result<Message, warp::Error>, stdin: &mut ChildStdin) -> Result<(), Option<(u8, String)>> {
    let msg = match msg {
        Ok(m) => m,
        Err(e) => return Err(Some(
            if let Some(e) = e.source()
                && let Some(e) = e.downcast_ref::<ws_error::Error>()
                && let ws_error::Error::Capacity(ws_error::CapacityError::MessageTooLong{size, ..}) = e
            {
                (TOO_LARGE, format!("received message of size {size}, greater than size limit {MAX_REQUEST_SIZE}"))
            } else {
                (INTERNAL_ERROR, format!("error reading from websocket: {e}"))
            }
        )),
    };
    if msg.is_close() {
        return Err(None)
    }
    if !msg.is_binary() {
        return Err(Some((UNSUPPORTED_DATA, format!("expected a binary message"))))
    }
    if let Err(e) = stdin.write(msg.as_bytes()).await {
        return Err(Some((INTERNAL_ERROR, format!("failed passing message on: {e}"))))
    }
    Ok(())
}

async fn invoke1() -> Result<(ChildStdin, Child), String> {
    let command = Command::new("/usr/local/lib/ATO/sandbox")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn();
    let mut child = match command {
        Ok(c) => c,
        Err(e) => return Err(format!("internal error: error spawning ATO/sandbox: {e}")),
    };
    let stdin = child
        .stdin
        .take()
        .expect("stdin should not have been taken");
    Ok((stdin, child))
}

async fn invoke2(child: Child) -> Result<(), (u8, String)> {
    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return Err((
                INTERNAL_ERROR,
                format!("internal error: error waiting for ATO/sandbox: {e}"),
            ))
        }
    };
    if !output.status.success() {
        let msg = std::string::String::from_utf8_lossy(&output.stderr[..]);
        let msg = msg.trim_end();
        eprintln!("{msg}");
        let code = match output.status.code() {
            Some(c) => c as u8,
            None => {
                eprintln!("internal error: error running ATO/sandbox: {}", output.status);
                INTERNAL_ERROR
            }
        };
        Err((code, msg.into()))
    } else {
        Ok(())
    }
}
