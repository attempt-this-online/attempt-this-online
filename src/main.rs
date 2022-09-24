extern crate lazy_static;

mod constants;
use crate::constants::*;
use futures_util::SinkExt;
use futures_util::StreamExt;
use futures_util::stream::{SplitSink, SplitStream};
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};
use tokio::task;
use warp::ws::*;
use warp::Filter;

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
                eprintln!("error reading from websocket: {e}");
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
                    eprintln!("error sending close code: {e}");
                }
            return;
        }
        if let Err((code, e)) = invoke(message.as_bytes(), &mut receiver, &mut sender).await {
            if let Err(e) = sender
                .send(Message::close_with(WEBSOCKET_BASE + code as u16, e))
                .await
            {
                // can't do anything but log it
                eprintln!("error sending close code: {e}");
            }
            return;
        };
    }
    if let Err(e) = sender.close().await {
        // can't do anything but log it
        eprintln!("error closing websocket: {e}");
    }
}

async fn invoke(
    input: &[u8],
    receiver: &mut SplitStream<WebSocket>,
    sender: &mut SplitSink<WebSocket, Message>,
) -> Result<(), (u8, String)> {
    let (mut stdin, mut child) =
        match invoke1(input).await {
            Ok(c) => c,
            Err(e) => {
                return Err((INTERNAL_ERROR, e))
            }
        };
    let mut stdout = child.stdout.take().expect("stdout should not have been taken");
    let mut wait = task::spawn(async { invoke2(child).await });
    let mut buf = [0u8; PIPE_BUF];
    loop {
        tokio::select! {
            res = &mut wait => return res.unwrap(),
            maybe_msg = receiver.next() => match maybe_msg {
                None => return wait.await.unwrap(),
                Some(msg) => handle_message(msg, &mut stdin).await?,
            },
            maybe_n = stdout.read(&mut buf) => match maybe_n {
                Err(e) => eprintln!("error reading message: {e}"),
                Ok(0) => { // EOF
                    return wait.await.unwrap()
                }
                Ok(n) => {
                    if let Err(e) = sender.send(Message::binary(&buf[..n])).await {
                        eprintln!("error sending to websocket: {e}")
                    }
                }
            }
        }
    }
}

async fn handle_message(msg: Result<Message, warp::Error>, stdin: &mut ChildStdin) -> Result<(), (u8, String)> {
    let msg = match msg {
        Ok(m) => m,
        Err(e) => return Err((INTERNAL_ERROR, format!("error getting websocket message: {e}"))),
    };
    if !msg.is_binary() {
        return Err((UNSUPPORTED_DATA, format!("expected a binary message")))
    }
    if let Err(e) = stdin.write(msg.as_bytes()).await {
        return Err((INTERNAL_ERROR, format!("failed passing message on: {e}")))
    }
    Ok(())
}

async fn invoke1(input: &[u8]) -> Result<(ChildStdin, Child), String> {
    let command = Command::new("/usr/local/lib/ATO/invoke")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn();
    let mut child = match command {
        Ok(c) => c,
        Err(e) => return Err(format!("internal error: error spawning ATO/invoke: {e}")),
    };
    let mut stdin = child
        .stdin
        .take()
        .expect("stdin should not have been taken");
    if let Err(e) = stdin.write_all(input).await {
        return Err(
            format!("internal error: error writing stdin of ATO/invoke: {e}"),
        );
    }
    Ok((stdin, child))
}

async fn invoke2(child: Child) -> Result<(), (u8, String)> {
    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return Err((
                INTERNAL_ERROR,
                format!("internal error: error waiting for ATO/invoke: {e}"),
            ))
        }
    };
    if !output.status.success() {
        let msg = std::string::String::from_utf8_lossy(&output.stderr[..]);
        eprintln!("invoke reported: {msg:#?}");
        let code = match output.status.code() {
            Some(c) => c as u8,
            None => {
                eprintln!("internal error: error running ATO/invoke: {}", output.status);
                INTERNAL_ERROR
            }
        };
        Err((code, msg.trim_end().into()))
    } else {
        Ok(())
    }
}
