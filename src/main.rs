use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use warp::Filter;
use warp::ws::*;

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

const MAX_REQUEST_SIZE: usize = 1 << 16; // 64KiB

#[tokio::main]
async fn main() {
    let execute = warp::path!("api" / "v0" / "ws" / "execute")
        .and(warp::ws())
        .map(|ws: Ws| {
            ws.max_message_size(MAX_REQUEST_SIZE).on_upgrade(handle_ws)
        })
        .with(warp::cors().allow_any_origin());
    warp::serve(execute).run(([127, 0, 0, 1], 8500)).await;
}

async fn handle_ws(websocket: WebSocket) {
    let (sender, receiver) = websocket.split();
    let mut sender = receiver.fold(sender, |mut sender, received| async {
        let message =
        match received {
            Ok(r) => { r }
            Err(e) => {
                // TODO: handle error
                eprintln!("error reading from websocket: {}", e);
                return sender;
            }
        };
        let request =
        match rmp_serde::from_slice::<Request>(message.as_bytes()) {
            Ok(r) => { r }
            Err(e) => {
                let msg = format!("error deserialising request: {}", e);
                eprintln!("{}", msg);
                return handle_ws_error(sender, msg, 1008).await;
            }
        };
        let response =
        match invoke(request).await {
            Ok(r) => { r }
            Err(e) => {
                let msg = format!("internal error: {}", e);
                return handle_ws_error(sender, msg, 1011).await;
            }
        };
        let encoded =
        match rmp_serde::to_vec_named(&response) {
            Ok(r) => { r }
            Err(e) => {
                eprintln!("error serialising response: {}", e);
                return sender;
            }
        };
        match sender.feed(Message::binary(encoded)).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error feeding websocket: {}", e);
            }
        };
        return sender;
    }).await;
    if let Err(e) = sender.flush().await {
        eprintln!("error flushing websocket: {}", e);
    }
    if let Err(e) = sender.close().await {
        eprintln!("error closing websocket: {}", e);
    }
}

async fn handle_ws_error(mut sender: SplitSink<WebSocket, Message>, error: String, code: u16) -> SplitSink<WebSocket, Message> {
    if let Err(e) = sender.send(Message::close_with(code, error)).await {
        // can't do anything but log it
        eprintln!("error sending close code: {}", e);
    }
    return sender;
}

async fn invoke(request: Request) -> Result<Response, String> {
    // TODO: implement invocations
    let _ = request;
    Ok(Response{stdout: "hello".into(), stderr: "goodbye".into()})
}
