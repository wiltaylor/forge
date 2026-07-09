//! GET /api/ws — WebSocket endpoint over the event bus.
//!
//! Server→client frames: `{"type":"event","topic":...,"data":...}`,
//! `{"type":"pong"}`, `{"type":"lagged"}`.
//! Client→server frames: `{"type":"subscribe","topics":[..]}` (empty/omitted
//! = all topics; the default before any subscribe is also all),
//! `{"type":"ping"}`.

use std::collections::HashSet;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use crate::events::EventBus;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Subscribe {
        #[serde(default)]
        topics: Vec<String>,
    },
    Ping,
}

pub(crate) async fn ws_handler(ws: WebSocketUpgrade, State(bus): State<EventBus>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, bus))
}

async fn handle_socket(mut socket: WebSocket, bus: EventBus) {
    let mut rx = bus.subscribe();
    // None = all topics (the default); Some(set) after a filtered subscribe.
    let mut topics: Option<HashSet<String>> = None;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => match serde_json::from_str::<ClientMsg>(&text) {
                        Ok(ClientMsg::Subscribe { topics: list }) => {
                            let set: HashSet<String> = list
                                .into_iter()
                                .filter(|t| !t.is_empty())
                                .collect();
                            topics = if set.is_empty() { None } else { Some(set) };
                        }
                        Ok(ClientMsg::Ping) => {
                            if socket
                                .send(Message::Text(r#"{"type":"pong"}"#.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::debug!(error = %e, "ignoring malformed ws client frame");
                        }
                    },
                    Message::Close(_) => break,
                    // axum answers protocol-level Ping frames automatically.
                    _ => {}
                }
            }
            ev = rx.recv() => {
                match ev {
                    Ok(ev) => {
                        if topics.as_ref().is_none_or(|t| t.contains(&ev.topic)) {
                            let frame = format!(
                                r#"{{"type":"event","topic":{},"data":{}}}"#,
                                serde_json::to_string(&ev.topic).unwrap_or_else(|_| "\"\"".into()),
                                ev.json,
                            );
                            if socket.send(Message::Text(frame.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!(missed = n, "ws consumer lagged");
                        if socket
                            .send(Message::Text(r#"{"type":"lagged"}"#.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}
