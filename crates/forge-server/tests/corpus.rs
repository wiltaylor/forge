//! The Rust HTTP driver for the contract corpus (`contract/corpus.json`).
//!
//! It builds the fixture the corpus describes, then runs every case that
//! declares `rust-http` under `applies`. The case list lives in the corpus, not
//! here — this file only knows how to turn an authored request into an HTTP
//! request and hand the response back to the matcher.
//!
//! Ordinary requests go through the router in process. Websocket cases need a
//! real handshake, so those get a real listener on an ephemeral port; the
//! router is the same one either way, so the event bus is shared.

mod common;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::Router;
use forge_contract::{
    interpolate, interpolate_value, match_value, users_env, write_fixture_files, Auth, AwaitEvent,
    Case, Connect, Corpus, Expect, Fixture, Kind, Step, Vars, RUST_HTTP,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

const WAIT: Duration = Duration::from_secs(5);
/// Driver-local: the corpus does not observe the signing secret.
const SECRET: &str = "0123456789abcdef0123456789abcdef";

#[tokio::test]
async fn corpus_rust_http() {
    let corpus = Corpus::load().expect("contract/corpus.json");

    // One server per fixture a case actually names, built the first time it
    // is needed: a fixture whose cases are all inapplicable here costs nothing.
    let mut harnesses: BTreeMap<&str, Harness> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut ran = 0;
    for case in corpus.cases_for(RUST_HTTP) {
        ran += 1;
        if !harnesses.contains_key(case.fixture.as_str()) {
            let harness = Harness::build(&corpus, &case.fixture).await;
            harnesses.insert(case.fixture.as_str(), harness);
        }
        let harness = &harnesses[case.fixture.as_str()];
        if let Err(why) = harness.run(case).await {
            failures.push(format!(
                "{} [{}]: {why}\n    ({})",
                case.id, case.fixture, case.title
            ));
        }
    }

    assert!(ran > 0, "no corpus case applies to {RUST_HTTP}");
    assert!(
        failures.is_empty(),
        "{} of {ran} contract cases failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// The fixture the corpus describes, plus the token every case borrows.
struct Harness {
    router: Router,
    vars: Vars,
    /// Kept alive: the doc store, components and frontend live under it.
    _dir: tempfile::TempDir,
}

impl Harness {
    async fn build(corpus: &Corpus, name: &str) -> Self {
        let fixture = corpus
            .fixtures
            .get(name)
            .unwrap_or_else(|| panic!("the corpus has no fixture {name:?}"));
        let vars = corpus.vars();
        let dir = tempfile::tempdir().expect("tempdir");
        let data = dir.path().join("data");
        let components = dir.path().join("components");
        let frontend = dir.path().join("frontend");
        for path in [&data, &components, &frontend] {
            std::fs::create_dir_all(path).expect("fixture dir");
        }
        write_fixture_files(&frontend, &fixture.frontend.files, &vars).expect("frontend");

        let mut app = forge_server::ForgeApp::new(interpolate(&fixture.app, &vars).expect("app"))
            .frontend_dir(&frontend);
        if let Some(fixture_components) = &fixture.components {
            // An authored manifest is written; an absent one is the point of
            // the fixture that leaves it out.
            if let Some(manifest) = &fixture_components.manifest {
                let manifest = interpolate_value(manifest, &vars).expect("manifest");
                std::fs::write(
                    components.join("manifest.json"),
                    serde_json::to_vec_pretty(&manifest).expect("manifest json"),
                )
                .expect("write manifest");
            }
            write_fixture_files(&components, &fixture_components.files, &vars).expect("components");
            app = app.with_components(&components);
        }
        if fixture.docstore {
            app = app.with_docstore(&data);
        }
        if let Some(events) = &fixture.events {
            app = app.with_events_config(events_config(events));
        }
        if fixture.auth.enabled {
            app = app.auth(auth_config(fixture, &vars));
        }
        for name in &fixture.actions {
            app = register_action(app, name);
        }
        let router = app.try_router().expect("fixture router");

        let mut harness = Self {
            router,
            vars,
            _dir: dir,
        };
        let token = harness.login(fixture).await;
        harness.vars.insert("token".into(), token);
        harness
    }

    /// The one thing the driver does that the corpus does not describe: it
    /// needs a token before it can run a case that carries one.
    async fn login(&self, fixture: &Fixture) -> String {
        if !fixture.auth.enabled {
            return String::new();
        }
        let user = fixture.auth.users.first().expect("fixture user");
        let body = json!({
            "username": interpolate(&user.name, &self.vars).expect("user"),
            "password": interpolate(&user.password, &self.vars).expect("password"),
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let res = self.send(req).await;
        assert_eq!(res.status, StatusCode::OK, "fixture login: {}", res.text);
        res.json()
            .expect("login envelope")
            .pointer("/data/token")
            .and_then(Value::as_str)
            .expect("login returns a token")
            .to_string()
    }

    async fn run(&self, case: &Case) -> Result<(), String> {
        match case.kind {
            Kind::Http => self.run_http(case).await,
            Kind::Sse => self.run_sse(case).await,
            Kind::Ws => self.run_ws(case).await,
        }
    }

    async fn run_http(&self, case: &Case) -> Result<(), String> {
        for (i, step) in case.steps.iter().enumerate() {
            let Step::Request { request, expect } = step else {
                return Err(format!("step {i}: kind `http` takes request steps only"));
            };
            self.run_request(request, expect.as_ref(), i).await?;
        }
        Ok(())
    }

    /// One request, checked. Every kind of case takes these, so all three
    /// runners come through here.
    async fn run_request(
        &self,
        request: &forge_contract::Request,
        expect: Option<&Expect>,
        step: usize,
    ) -> Result<(), String> {
        let res = self.send(self.build_request(request)?).await;
        self.check(expect, &res, step)
    }

    /// The first step opens the stream and its response is checked for status
    /// and headers; later request steps go out beside it, on the same router.
    async fn run_sse(&self, case: &Case) -> Result<(), String> {
        let mut stream: Option<SseStream> = None;
        for (i, step) in case.steps.iter().enumerate() {
            match step {
                Step::Request { request, expect } if stream.is_none() => {
                    let res = self
                        .router
                        .clone()
                        .oneshot(self.build_request(request)?)
                        .await
                        .map_err(|e| format!("step {i}: {e}"))?;
                    let (parts, body) = res.into_parts();
                    let head = Response {
                        status: parts.status,
                        headers: parts.headers,
                        text: String::new(),
                    };
                    check_status_and_headers(expect.as_ref(), &head, i, &self.vars)?;
                    stream = Some(SseStream::new(body));
                }
                Step::Request { request, expect } => {
                    self.run_request(request, expect.as_ref(), i).await?;
                }
                Step::AwaitEvent { await_event } => {
                    let stream = stream.as_mut().expect("stream opened by the first step");
                    self.expect_event(stream, await_event)
                        .await
                        .map_err(|e| format!("step {i}: {e}"))?;
                }
                Step::AwaitHeartbeat { await_heartbeat } => {
                    let stream = stream.as_mut().expect("stream opened by the first step");
                    self.expect_heartbeat(stream, await_heartbeat)
                        .await
                        .map_err(|e| format!("step {i}: {e}"))?;
                }
                _ => return Err(format!("step {i}: not a step a stream can take")),
            }
        }
        Ok(())
    }

    async fn expect_event(
        &self,
        stream: &mut SseStream,
        expected: &AwaitEvent,
    ) -> Result<(), String> {
        let (topic, data) = stream.next_event().await?;
        let wanted = interpolate(&expected.topic, &self.vars)?;
        if topic != wanted {
            return Err(format!("expected topic {wanted:?}, got {topic:?}"));
        }
        match_value(&expected.data, &data, &self.vars)
    }

    /// The *next* block, not the next heartbeat: a driver that read past an
    /// event to find one would pass while the stream carried something the
    /// contract does not allow.
    async fn expect_heartbeat(
        &self,
        stream: &mut SseStream,
        expected: &Value,
    ) -> Result<(), String> {
        match stream.next_block().await? {
            SseBlock::Heartbeat(text) => match_value(expected, &Value::String(text), &self.vars),
            SseBlock::Event(topic, data) => Err(format!(
                "expected a heartbeat, got the event {topic:?} carrying {data}"
            )),
        }
    }

    async fn run_ws(&self, case: &Case) -> Result<(), String> {
        let addr = self.serve().await;
        let mut socket = None;
        for (i, step) in case.steps.iter().enumerate() {
            let at = |e: String| format!("step {i}: {e}");
            match step {
                Step::Connect { connect, expect } => {
                    match (self.connect(addr, connect).await, expect) {
                        (Ok(ws), None) => socket = Some(ws),
                        (Ok(_), Some(_)) => {
                            return Err(at("handshake succeeded, expected a refusal".into()))
                        }
                        (Err(res), Some(expect)) => self.check(Some(expect), &res, i)?,
                        (Err(res), None) => {
                            return Err(at(format!(
                                "handshake refused: {} {}",
                                res.status, res.text
                            )))
                        }
                    }
                }
                Step::Send { send } => {
                    let frame = interpolate_value(send, &self.vars)?;
                    socket
                        .as_mut()
                        .ok_or_else(|| at("no open socket".into()))?
                        .send(Message::text(frame.to_string()))
                        .await
                        .map_err(|e| at(e.to_string()))?;
                }
                Step::AwaitFrame { await_frame } => {
                    let ws = socket.as_mut().ok_or_else(|| at("no open socket".into()))?;
                    let frame = next_frame(ws).await.map_err(at)?;
                    match_value(await_frame, &frame, &self.vars)
                        .map_err(|e| format!("step {i}: {e}"))?;
                }
                Step::Request { request, expect } => {
                    self.run_request(request, expect.as_ref(), i).await?;
                }
                Step::AwaitEvent { .. } | Step::AwaitHeartbeat { .. } => {
                    return Err(at("a socket awaits frames".into()))
                }
            }
        }
        if let Some(mut ws) = socket {
            ws.close(None).await.ok();
        }
        Ok(())
    }

    /// A real listener, for the cases that need a real handshake. Router
    /// clones share their state, so this server publishes onto the same bus as
    /// the in-process requests.
    async fn serve(&self) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = self.router.clone();
        tokio::spawn(async move {
            axum::serve(listener, router).await.ok();
        });
        addr
    }

    async fn connect(
        &self,
        addr: SocketAddr,
        connect: &Connect,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Response,
    > {
        let path = self
            .uri(&connect.path, &connect.query, connect.auth)
            .expect("connect uri");
        let url = format!("ws://{addr}{path}");
        match tokio_tungstenite::connect_async(url).await {
            Ok((ws, _)) => Ok(ws),
            Err(tokio_tungstenite::tungstenite::Error::Http(res)) => {
                let (parts, body) = res.into_parts();
                Err(Response {
                    status: parts.status,
                    headers: parts.headers,
                    text: body
                        .map(|b| String::from_utf8_lossy(&b).into_owned())
                        .unwrap_or_default(),
                })
            }
            Err(e) => Err(Response {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                headers: HeaderMap::new(),
                text: e.to_string(),
            }),
        }
    }

    fn build_request(&self, request: &forge_contract::Request) -> Result<Request<Body>, String> {
        let uri = self.uri(&request.path, &request.query, request.auth)?;
        let mut builder = Request::builder().method(request.method.as_str()).uri(&uri);
        for (name, value) in &request.headers {
            builder = builder.header(name, interpolate(value, &self.vars)?);
        }
        if request.auth == Auth::Bearer {
            builder = builder.header("authorization", format!("Bearer {}", self.token()));
        }
        let body = match &request.body {
            Some(body) => {
                builder = builder.header("content-type", "application/json");
                Body::from(interpolate_value(body, &self.vars)?.to_string())
            }
            None => Body::empty(),
        };
        builder
            .body(body)
            .map_err(|e| format!("cannot build a request for {uri}: {e}"))
    }

    /// Path plus query. The path is authored as it goes on the wire, so it is
    /// used verbatim; query values are encoded here.
    fn uri(
        &self,
        path: &str,
        query: &BTreeMap<String, String>,
        auth: Auth,
    ) -> Result<String, String> {
        let mut pairs = Vec::new();
        for (name, value) in query {
            pairs.push(format!(
                "{name}={}",
                encode(&interpolate(value, &self.vars)?)
            ));
        }
        if auth == Auth::Query {
            pairs.push(format!("token={}", encode(self.token())));
        }
        let path = interpolate(path, &self.vars)?;
        Ok(if pairs.is_empty() {
            path
        } else {
            format!("{path}?{}", pairs.join("&"))
        })
    }

    fn token(&self) -> &str {
        self.vars.get("token").map(String::as_str).unwrap_or("")
    }

    async fn send(&self, req: Request<Body>) -> Response {
        let (status, headers, text) = common::send_raw(&self.router, req).await;
        Response {
            status,
            headers,
            text,
        }
    }

    fn check(&self, expect: Option<&Expect>, res: &Response, step: usize) -> Result<(), String> {
        check_status_and_headers(expect, res, step, &self.vars)?;
        let Some(expect) = expect else { return Ok(()) };
        let at = |e: String| format!("step {step}: {e}");
        if let Some(want) = &expect.body {
            let body = res
                .json()
                .ok_or_else(|| at(format!("body is not JSON: {:?}", res.text)))?;
            match_value(want, &body, &self.vars).map_err(at)?;
        }
        if let Some(want) = &expect.text {
            match_value(want, &Value::String(res.text.clone()), &self.vars).map_err(at)?;
        }
        Ok(())
    }
}

fn check_status_and_headers(
    expect: Option<&Expect>,
    res: &Response,
    step: usize,
    vars: &Vars,
) -> Result<(), String> {
    let Some(expect) = expect else { return Ok(()) };
    if res.status.as_u16() != expect.status {
        return Err(format!(
            "step {step}: expected status {}, got {} ({})",
            expect.status,
            res.status.as_u16(),
            res.text.trim()
        ));
    }
    for (name, want) in &expect.headers {
        let got = res
            .headers
            .get(name)
            .ok_or_else(|| format!("step {step}: no {name} header"))?
            .to_str()
            .map_err(|_| format!("step {step}: {name} is not text"))?;
        match_value(want, &Value::String(got.to_string()), vars)
            .map_err(|e| format!("step {step}: header {name}: {e}"))?;
    }
    Ok(())
}

struct Response {
    status: StatusCode,
    headers: HeaderMap,
    text: String,
}

impl Response {
    fn json(&self) -> Option<Value> {
        serde_json::from_str(&self.text).ok()
    }
}

/// One block off a live server-sent-events body.
enum SseBlock {
    /// An `event:`/`data:` pair.
    Event(String, Value),
    /// The comment that holds an idle stream open, its text without the colon.
    Heartbeat(String),
}

/// Reads blocks off a live server-sent-events body.
struct SseStream {
    body: Body,
    buffer: String,
}

impl SseStream {
    fn new(body: Body) -> Self {
        Self {
            body,
            buffer: String::new(),
        }
    }

    /// The next event. Comment heartbeats are not events, so they are stepped
    /// over — `await_heartbeat` is the step that sees one.
    async fn next_event(&mut self) -> Result<(String, Value), String> {
        loop {
            if let SseBlock::Event(topic, data) = self.next_block().await? {
                return Ok((topic, data));
            }
        }
    }

    async fn next_block(&mut self) -> Result<SseBlock, String> {
        loop {
            while let Some(block) = self.take_block() {
                if let Some(block) = parse_sse_block(&block)? {
                    return Ok(block);
                }
            }
            let frame = timeout(WAIT, self.body.frame())
                .await
                .map_err(|_| "timed out waiting for the stream".to_string())?
                .ok_or_else(|| "the stream ended".to_string())?
                .map_err(|e| format!("stream error: {e}"))?;
            if let Some(data) = frame.data_ref() {
                self.buffer.push_str(&String::from_utf8_lossy(data));
            }
        }
    }

    fn take_block(&mut self) -> Option<String> {
        let end = self.buffer.find("\n\n")?;
        let block = self.buffer[..end].to_string();
        self.buffer.drain(..end + 2);
        Some(block)
    }
}

/// `None` for a block that is neither an event nor a heartbeat.
fn parse_sse_block(block: &str) -> Result<Option<SseBlock>, String> {
    let mut topic = None;
    let mut data = None;
    let mut comment = None;
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            topic = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            data = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix(':') {
            comment.get_or_insert_with(|| rest.trim().to_string());
        }
    }
    match (topic, data, comment) {
        (Some(topic), Some(data), _) => {
            let value = serde_json::from_str(&data)
                .map_err(|e| format!("event data is not JSON: {data:?} ({e})"))?;
            Ok(Some(SseBlock::Event(topic, value)))
        }
        (_, _, Some(comment)) => Ok(Some(SseBlock::Heartbeat(comment))),
        _ => Ok(None),
    }
}

async fn next_frame(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<Value, String> {
    loop {
        let message = timeout(WAIT, ws.next())
            .await
            .map_err(|_| "timed out waiting for a frame".to_string())?
            .ok_or_else(|| "the socket closed".to_string())?
            .map_err(|e| format!("socket error: {e}"))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(&text)
                    .map_err(|e| format!("frame is not JSON: {text:?} ({e})"))
            }
            // Ping/pong at the protocol level is not a contract frame.
            Message::Ping(_) | Message::Pong(_) => continue,
            other => return Err(format!("expected a text frame, got {other:?}")),
        }
    }
}

/// The fixture's users, configured the way a deployment configures them:
/// through the `FORGE_AUTH_USERS` parser. Handing the store a name and a
/// secret directly would step over the parse, which is where an argon2 hash's
/// commas land.
fn auth_config(fixture: &Fixture, vars: &Vars) -> forge_server::AuthConfig {
    let raw = users_env(&fixture.auth, vars).expect("fixture users");
    let mut config = forge_server::AuthConfig::new(SECRET);
    config.users = forge_server::auth::parse_users(&raw)
        .expect("the fixture users are not a valid FORGE_AUTH_USERS value");
    // The variable carries no roles and the fixture does, so they go back on
    // by name.
    for user in &fixture.auth.users {
        let name = interpolate(&user.name, vars).expect("user name");
        let stored = config
            .users
            .iter_mut()
            .find(|u| u.name == name)
            .expect("every fixture user survives the parse");
        stored.roles = user.roles.clone();
    }
    config
}

fn events_config(events: &forge_contract::FixtureEvents) -> forge_server::EventsConfig {
    let mut config = forge_server::EventsConfig::default();
    if let Some(buffer) = events.buffer {
        config = config.buffer(buffer);
    }
    if let Some(secs) = events.heartbeat_s {
        config = config.heartbeat(Duration::from_secs_f64(secs));
    }
    config
}

/// The behaviour each fixture action must have, per `contract/README.md`. An
/// action the corpus names and this driver does not know is a failure, not a
/// silent gap.
fn register_action(app: forge_server::ForgeApp, name: &str) -> forge_server::ForgeApp {
    match name {
        "echo" => app.action("echo", |payload, _ctx| async move { Ok(payload) }),
        "publish" => app.action("publish", |payload: Value, ctx| async move {
            let topic = payload
                .get("topic")
                .and_then(Value::as_str)
                .unwrap_or("misc")
                .to_string();
            let data = payload.get("data").cloned().unwrap_or(Value::Null);
            ctx.events.publish(&topic, data);
            Ok(json!({"published": true, "topic": topic}))
        }),
        // Publishes without awaiting anything, so no subscriber gets a turn
        // in between: that is what makes overrunning a bounded buffer a
        // certainty rather than a race.
        "flood" => app.action("flood", |payload: Value, ctx| async move {
            let topic = payload
                .get("topic")
                .and_then(Value::as_str)
                .unwrap_or("misc")
                .to_string();
            let count = payload.get("count").and_then(Value::as_u64).unwrap_or(0);
            for n in 0..count {
                ctx.events.publish(&topic, json!({"n": n}));
            }
            Ok(json!({"published": count, "topic": topic}))
        }),
        other => {
            panic!("the corpus fixture wants an action this driver has no behaviour for: {other:?}")
        }
    }
}

/// Percent-encode a query value. Paths are authored already encoded, so this
/// only ever sees values — a handful of tokens and topic names, which is not
/// worth a dependency.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}
