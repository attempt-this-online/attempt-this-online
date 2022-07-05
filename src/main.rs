use futures_util::SinkExt;
use futures_util::StreamExt;
use serde::{Serialize};
use warp::Filter;

#[derive(Serialize)]
struct Response {
    message: String,
}

#[tokio::main]
async fn main() {
    let execute = warp::path!("api" / "v0" / "ws" / "execute")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(handle_ws)
        })
        .with(warp::cors().allow_any_origin());
    warp::serve(execute).run(([127, 0, 0, 1], 8500)).await;
}

async fn handle_ws(websocket: warp::ws::WebSocket) {
    let (mut sender, _) = websocket.split();
    sender.send(
        warp::ws::Message::binary(
            rmp_serde::to_vec_named(
                &Response{message: "Hello, World!".into()}
            ).unwrap()
        )
    ).await.unwrap();
}
