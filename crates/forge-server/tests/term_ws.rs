//! /api/term session behaviour: local PTY echo, resize, exit codes and
//! config gating. Route mounting + auth are covered by widgets_ws.rs, so
//! these routers run auth-disabled.
#![cfg(feature = "term")]

use std::time::Duration;

use forge_server::widgets::TermConfig;
use forge_server::ForgeApp;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::time::{timeout, timeout_at, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

const WAIT: Duration = Duration::from_secs(10);

async fn spawn(router: axum::Router) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

async fn connect_term(config: TermConfig) -> Ws {
    let router = ForgeApp::new("t").with_term_config(config).router();
    let addr = spawn(router).await;
    let (ws, _) = connect_async(format!("ws://{addr}/api/term"))
        .await
        .expect("ws connect");
    ws
}

async fn send_json(ws: &mut Ws, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

async fn next_msg(ws: &mut Ws) -> Message {
    timeout(WAIT, ws.next())
        .await
        .expect("timed out waiting for ws frame")
        .expect("ws closed")
        .expect("ws error")
}

/// Read frames until a text control frame arrives, returning it parsed.
/// Binary tty output seen on the way accumulates into `sink`.
async fn next_ctrl(ws: &mut Ws, sink: &mut Vec<u8>) -> Value {
    loop {
        match next_msg(ws).await {
            Message::Text(t) => return serde_json::from_str(&t).unwrap(),
            Message::Binary(b) => sink.extend_from_slice(&b),
            _ => {}
        }
    }
}

/// Accumulate binary output until it contains `needle`.
async fn read_until(ws: &mut Ws, buf: &mut Vec<u8>, needle: &str) {
    let deadline = Instant::now() + WAIT;
    while !String::from_utf8_lossy(buf).contains(needle) {
        let msg = timeout_at(deadline, ws.next())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "timed out waiting for {needle:?}; output so far: {:?}",
                    String::from_utf8_lossy(buf)
                )
            })
            .expect("ws closed")
            .expect("ws error");
        if let Message::Binary(b) = msg {
            buf.extend_from_slice(&b);
        }
    }
}

fn sh_config() -> TermConfig {
    TermConfig {
        shell: Some("/bin/sh".into()),
        ..TermConfig::default()
    }
}

#[tokio::test]
async fn local_shell_runs_commands_and_reports_exit_code() {
    let mut ws = connect_term(sh_config()).await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"local","cols":80,"rows":24}),
    )
    .await;

    let mut out = Vec::new();
    let ready = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(ready["type"], "ready");

    // printf assembles the marker so the local echo of the typed command
    // can't satisfy the assertion.
    ws.send(Message::Binary(
        b"printf 'forge%s\\n' -pty-ok\n".to_vec().into(),
    ))
    .await
    .unwrap();
    read_until(&mut ws, &mut out, "forge-pty-ok").await;

    ws.send(Message::Binary(b"exit 7\n".to_vec().into()))
        .await
        .unwrap();
    let exit = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(exit["type"], "exit");
    assert_eq!(exit["code"], 7);
}

#[tokio::test]
async fn resize_reaches_the_pty() {
    let mut ws = connect_term(sh_config()).await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"local","cols":80,"rows":24}),
    )
    .await;
    let mut out = Vec::new();
    assert_eq!(next_ctrl(&mut ws, &mut out).await["type"], "ready");

    // Frames are processed in order: resize lands before stty runs.
    send_json(&mut ws, json!({"type":"resize","cols":100,"rows":40})).await;
    ws.send(Message::Binary(b"stty size\n".to_vec().into()))
        .await
        .unwrap();
    read_until(&mut ws, &mut out, "40 100").await;
}

#[tokio::test]
async fn local_disabled_yields_error() {
    let mut ws = connect_term(TermConfig {
        allow_local: false,
        ..sh_config()
    })
    .await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"local","cols":80,"rows":24}),
    )
    .await;
    let mut out = Vec::new();
    let msg = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("disabled"));
}

#[tokio::test]
async fn ssh_disabled_yields_error() {
    let mut ws = connect_term(TermConfig {
        allow_ssh: false,
        ..sh_config()
    })
    .await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"ssh","host":"h","username":"u","password":"p",
               "cols":80,"rows":24}),
    )
    .await;
    let mut out = Vec::new();
    let msg = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("disabled"));
}

#[tokio::test]
async fn binary_before_start_yields_error() {
    let mut ws = connect_term(sh_config()).await;
    ws.send(Message::Binary(b"ls\n".to_vec().into()))
        .await
        .unwrap();
    let mut out = Vec::new();
    let msg = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("start"));
}

#[cfg(feature = "term-ssh")]
#[tokio::test]
async fn ssh_host_not_in_allowlist_yields_error() {
    let mut ws = connect_term(TermConfig {
        allow_hosts: Some(vec!["allowed.example".into()]),
        ..sh_config()
    })
    .await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"ssh","host":"other.example","username":"u",
               "password":"p","cols":80,"rows":24}),
    )
    .await;
    let mut out = Vec::new();
    let msg = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("allowed hosts"));
}

#[cfg(feature = "term-ssh")]
#[tokio::test]
async fn ssh_missing_target_yields_error() {
    let mut ws = connect_term(sh_config()).await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"ssh","cols":80,"rows":24}),
    )
    .await;
    let mut out = Vec::new();
    let msg = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("requires"));
}

/// Live SSH round-trip against a real server. Run with:
/// `FORGE_TEST_SSH_ADDR=host:port FORGE_TEST_SSH_USER=u FORGE_TEST_SSH_PASS=p \
///  cargo test -p forge-server --features term-ssh -- --ignored ssh_live`
#[cfg(feature = "term-ssh")]
#[tokio::test]
#[ignore = "needs FORGE_TEST_SSH_ADDR + FORGE_TEST_SSH_USER + FORGE_TEST_SSH_PASS"]
async fn ssh_live_shell_round_trip() {
    let addr = std::env::var("FORGE_TEST_SSH_ADDR").expect("FORGE_TEST_SSH_ADDR");
    let user = std::env::var("FORGE_TEST_SSH_USER").expect("FORGE_TEST_SSH_USER");
    let pass = std::env::var("FORGE_TEST_SSH_PASS").expect("FORGE_TEST_SSH_PASS");
    let (host, port) = match addr.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().expect("port")),
        None => (addr, 22),
    };

    let mut ws = connect_term(TermConfig::default()).await;
    send_json(
        &mut ws,
        json!({"type":"start","mode":"ssh","host":host,"port":port,"username":user,
               "password":pass,"cols":80,"rows":24}),
    )
    .await;

    let mut out = Vec::new();
    let ready = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(ready["type"], "ready", "ssh connect failed: {ready}");

    ws.send(Message::Binary(
        b"printf 'forge%s\\n' -ssh-ok\n".to_vec().into(),
    ))
    .await
    .unwrap();
    read_until(&mut ws, &mut out, "forge-ssh-ok").await;

    ws.send(Message::Binary(b"exit\n".to_vec().into()))
        .await
        .unwrap();
    let exit = next_ctrl(&mut ws, &mut out).await;
    assert_eq!(exit["type"], "exit");
}
