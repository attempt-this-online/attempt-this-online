extern crate lazy_static;

mod codes;
use crate::codes::*;
use futures_util::SinkExt;
use futures_util::StreamExt;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use warp::ws::*;
use warp::Filter;

const MAX_REQUEST_SIZE: usize = 1 << 16; // 64KiB

#[tokio::main]
async fn main() {
    let execute = warp::path!("api" / "v0" / "ws" / "execute")
        .and(warp::ws())
        .map(|ws: Ws| ws.max_message_size(MAX_REQUEST_SIZE).on_upgrade(handle_ws))
        .with(warp::cors().allow_any_origin());
    warp::serve(execute).run(([127, 0, 0, 1], 8500)).await;
}

async fn handle_ws(websocket: WebSocket) {
    let (mut sender, mut receiver) = websocket.split();
    while let Some(received) = receiver.next().await {
        let message = match received {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error reading from websocket: {}", e);
                continue;
            }
        };
        if message.is_close() {
            break;
        }
        let response = match invoke(message.as_bytes()).await {
            Ok(r) => r,
            Err((code, e)) => {
                if let Err(e) = sender
                    .send(Message::close_with(WEBSOCKET_BASE + code as u16, e))
                    .await
                {
                    // can't do anything but log it
                    eprintln!("error sending close code: {}", e);
                }
                return;
            }
        };
        match sender.send(Message::binary(response)).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error sending to websocket: {}", e);
            }
        }
    }
    if let Err(e) = sender.close().await {
        // can't do anything but log it
        eprintln!("error closing websocket: {}", e);
    }
}

async fn invoke(input: &[u8]) -> Result<Vec<u8>, (u8, String)> {
    let command = Command::new("ATO_invoke")
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn();
    let mut child = match command {
        Ok(c) => c,
        Err(e) => return Err((INTERNAL_ERROR, format!("internal error: error spawning ATO_invoke: {}", e))),
    };
    let mut stdin = child
        .stdin
        .take()
        .expect("stdin should not have been taken");
    if let Err(e) = stdin.write_all(input).await {
        return Err((
            INTERNAL_ERROR,
            format!("internal error: error writing stdin of ATO_invoke: {}", e),
        ));
    }
    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return Err((
                INTERNAL_ERROR,
                format!("internal error: error waiting for ATO_invoke: {}", e),
            ))
        }
    };
    if !output.status.success() {
        let msg = std::string::String::from_utf8_lossy(&output.stdout[..]);
        let code = match output.status.code() {
            Some(c) => c as u8,
            None => {
                eprintln!("internal error: error running ATO_invoke: {}", output.status);
                INTERNAL_ERROR
            }
        };
        Err((code, msg.trim_end().into()))
    } else {
        Ok(output.stdout)
    }
}
