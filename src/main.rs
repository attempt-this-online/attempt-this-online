extern crate lazy_static;

mod codes;
use crate::codes::*;
use futures_util::SinkExt;
use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task;
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
        } else if !message.is_binary() {
                if let Err(e) = sender
                    .send(Message::close_with(WEBSOCKET_BASE + UNSUPPORTED_DATA as u16, "expected a binary message"))
                    .await
                {
                    // can't do anything but log it
                    eprintln!("error sending close code: {}", e);
                }
            return;
        }
        let response = match invoke(message.as_bytes(), &mut receiver).await {
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

async fn invoke(input: &[u8], ws: &mut SplitStream<WebSocket>) -> Result<Vec<u8>, (u8, String)> {
    let (mut stdin, child) =
        match invoke1(input).await {
            Ok(c) => c,
            Err(e) => {
                return Err((INTERNAL_ERROR, e))
            }
        };
    let mut wait = task::spawn(async { invoke2(child).await });
    loop {
        tokio::select! {
            res = &mut wait => return res.unwrap(),
            maybe_msg = ws.next() => match maybe_msg {
                None => return wait.await.unwrap(),
                Some(msg) => handle_message(msg, &mut stdin).await?,
            }
        }
    }
}

async fn handle_message(msg: Result<Message, warp::Error>, stdin: &mut ChildStdin) -> Result<(), (u8, String)> {
    let msg = match msg {
        Ok(m) => m,
        Err(e) => return Err((INTERNAL_ERROR, format!("error getting websocket message: {}", e))),
    };
    if !msg.is_binary() {
        return Err((UNSUPPORTED_DATA, format!("expected a binary message")))
    }
    if let Err(e) = stdin.write(msg.as_bytes()).await {
        return Err((INTERNAL_ERROR, format!("failed passing message on: {}", e)))
    }
    Ok(())
}

async fn invoke1(input: &[u8]) -> Result<(ChildStdin, Child), String> {
    let command = Command::new("ATO_invoke")
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn();
    let mut child = match command {
        Ok(c) => c,
        Err(e) => return Err(format!("internal error: error spawning ATO_invoke: {}", e)),
    };
    let mut stdin = child
        .stdin
        .take()
        .expect("stdin should not have been taken");
    if let Err(e) = stdin.write_all(input).await {
        return Err(
            format!("internal error: error writing stdin of ATO_invoke: {}", e),
        );
    }
    Ok((stdin, child))
}

async fn invoke2(child: Child) -> Result<Vec<u8>, (u8, String)> {
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
