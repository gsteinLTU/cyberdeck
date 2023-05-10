use anyhow::Result;
use axum::{response::Html, response::IntoResponse, routing::get, routing::post, Json, Router, http::{Method, header, Response, StatusCode, HeaderValue}, body::{self, Full}};
use cyberdeck::*;
use tokio::time::{sleep, Duration};
use tower_http::cors::{Any, CorsLayer};
use std::{net::SocketAddr, sync::Arc};
use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;

static CLIENTS: Lazy<DashMap<u128, DashMap<String, Arc<RTCDataChannel>>>> = Lazy::new(|| {
    DashMap::new()
});

static ROOMS: Lazy<DashMap<String, DashSet<u128>>> = Lazy::new(|| {
    DashMap::new()
});

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/pkg/client_wasm.js", get(js))
        .route("/pkg/client_wasm_bg.wasm", get(wasm))
        .route("/connect", post(connect))
	.layer(CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any)
	.allow_headers([header::CONTENT_TYPE]));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Running server on http://localhost:3000 ...");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn connect(Json(offer): Json<String>) -> impl IntoResponse {
    match start_peer_connection(offer).await {
        Ok(answer) => Ok(Json(answer)),
        Err(_) => Err("failed to connect"),
    }
}

async fn start_peer_connection(offer: String) -> Result<String> {
    let mut peer = Peer::new(|peer_id, e| async move {
        if !CLIENTS.contains_key(&peer_id) {
            CLIENTS.insert(peer_id, DashMap::new());
        }

        match e {
            PeerEvent::DataChannelMessage(c, m) => {
                println!(
                    "{}::Recieved a message from channel {} with id {}!",
                    peer_id,
                    c.label(),
                    c.id()
                );
                let msg_str = String::from_utf8(m.data.to_vec()).unwrap();
                println!(
                    "{}::Message from DataChannel '{}': {}",
                    peer_id,
                    c.label(),
                    msg_str
                );

                // Lobby channel handles specific commands only
                if c.label() == "lobby" {
                    if msg_str.starts_with("/join") {
                        let room_name = msg_str.split(" ").nth(1);

                        if let Some(room_name) = room_name {
                            if !ROOMS.contains_key(room_name) {
                                ROOMS.insert(room_name.to_owned(), DashSet::new());
                            }

                            ROOMS.get_mut(room_name).unwrap().insert(peer_id);
                        } else {
                            println!("{}::Invalid attempt to join room", peer_id);
                        }
                    }
                }

                // Send to room participants
                if let Some(room) = ROOMS.get(c.label()){ 
                    for client_id in room.iter() {
                        if let Some(channel) = CLIENTS.get(&client_id).and_then(| client | {
                            client.get(c.label()).and_then(|channel| {
                                Some(channel.clone())
                            })
                        }) {
                            channel.send_text(format!("{}: {}", peer_id, msg_str)).await;
                        }
                    }
                }
            }
            PeerEvent::DataChannelStateChange(c) => {
                if c.ready_state() == RTCDataChannelState::Open {
                    println!("{}::DataChannel '{}'", peer_id, c.label());
                    c.send_text("Connected to client!".to_string())
                        .await
                        .unwrap();
                    
                    CLIENTS.get(&peer_id).unwrap().value().insert(c.label().to_owned(), c.clone());
                } else if c.ready_state() == RTCDataChannelState::Closed {
                    println!("{}::DataChannel '{}'", peer_id, c.label());
                }
            }
            PeerEvent::PeerConnectionStateChange(s) => {
                match s {
                    RTCPeerConnectionState::Closed | RTCPeerConnectionState::Disconnected | RTCPeerConnectionState::Failed => {
                        CLIENTS.remove(&peer_id);
                    },
                    _ => {}
                }
                println!("{}::Peer connection state: {} ", peer_id, s);
            }
        }
    })
    .await?;

    let answer = peer.receive_offer(&offer).await?;

    // move cyberdeck to another thread to keep it alive
    tokio::spawn(async move {
        while peer.connection_state() != RTCPeerConnectionState::Closed
            && peer.connection_state() != RTCPeerConnectionState::Disconnected
            && peer.connection_state() != RTCPeerConnectionState::Failed
        {
            // keep the connection alive while not in invalid state
            sleep(Duration::from_millis(1000)).await;
        }
        // because we moved cyberdeck ownership into here gets dropped here and will stop all channels
    });

    Ok(answer)
}

// basic handler that responds with a static string
async fn root() -> impl IntoResponse {
    Html(include_str!("../../client-wasm/index.html"))
}

// basic handler that responds with a static string
async fn js() -> impl IntoResponse {
    Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str("application/javascript").unwrap(),
            )
            .body(body::boxed(Full::from(std::fs::read("../client-wasm/pkg/client_wasm.js").unwrap())))
            .unwrap()
}

// basic handler that responds with a static string
async fn wasm() -> impl IntoResponse {
    Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str("application/wasm").unwrap(),
            )
            .body(body::boxed(Full::from(std::fs::read("../client-wasm/pkg/client_wasm_bg.wasm").unwrap())))
            .unwrap()
}