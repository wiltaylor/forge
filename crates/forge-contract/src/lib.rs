//! The Forge contract corpus, and the two things every Rust driver of it
//! needs: a typed reading of `contract/corpus.json`, and a matcher for the
//! expectations it holds.
//!
//! Nothing here knows about HTTP. A driver supplies the transport — it builds
//! the fixture, turns a [`Request`] into whatever its transport sends, and
//! hands the response back to [`match_value`] and [`Expect`].
//!
//! ```
//! # use forge_contract::Corpus;
//! let corpus = Corpus::load().unwrap();
//! for case in corpus.cases_for("rust-http") {
//!     assert!(!case.steps.is_empty());
//! }
//! ```

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

mod matcher;

pub use matcher::{interpolate, interpolate_value, match_value, Vars};

/// The corpus as authored, verbatim.
pub const CORPUS_JSON: &str = include_str!("../../../contract/corpus.json");

/// Transport id of the Rust HTTP driver.
pub const RUST_HTTP: &str = "rust-http";

/// Transport id of the Rust IPC driver.
pub const RUST_IPC: &str = "rust-ipc";

/// Write a fixture file group — `components.files`, `frontend.files` — into
/// `dir`, interpolating both the names and the contents.
///
/// Every driver provisions the same fixture, so the two Rust ones share this
/// rather than each holding its own copy of what `${}` in a filename means.
pub fn write_fixture_files(
    dir: &std::path::Path,
    files: &BTreeMap<String, String>,
    vars: &Vars,
) -> Result<(), String> {
    for (name, content) in files {
        let name = interpolate(name, vars)?;
        let content = interpolate(content, vars)?;
        std::fs::write(dir.join(&name), content)
            .map_err(|e| format!("cannot write the fixture file {name:?}: {e}"))?;
    }
    Ok(())
}

/// Name of the fixture a case runs against unless it names another.
pub const DEFAULT_FIXTURE: &str = "default";

/// The fixture's users as `FORGE_AUTH_USERS` carries them: comma-separated
/// `name:secret` entries, the first colon splitting the two.
///
/// Every driver configures its backend through that variable's own parser
/// rather than handing the user store a name and a secret directly. The corpus
/// holds a user whose stored secret is an argon2 PHC hash, and a PHC hash
/// carries commas in its parameters — which is the separator the variable uses,
/// and the one place the two backends have already diverged in the field.
pub fn users_env(auth: &FixtureAuth, vars: &Vars) -> Result<String, String> {
    let mut entries = Vec::with_capacity(auth.users.len());
    for user in &auth.users {
        entries.push(format!(
            "{}:{}",
            interpolate(&user.name, vars)?,
            interpolate(user.stored_secret(), vars)?
        ));
    }
    Ok(entries.join(","))
}

/// One authored contract corpus.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    /// Contract version the cases describe (`docs/api-contract.md`).
    pub contract_version: String,
    /// Every transport a case must account for.
    pub transports: Vec<String>,
    /// Substitution table for `${name}` in paths, bodies and expectations.
    pub vars: BTreeMap<String, String>,
    /// The servers a driver builds, by name. `default` is the one a case
    /// runs against unless it names another.
    pub fixtures: BTreeMap<String, Fixture>,
    /// The contract cases, in authored order.
    pub cases: Vec<Case>,
}

/// The server state the cases assume. See `contract/README.md` for the
/// behaviour each named action must have.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// The application name the backend reports.
    pub app: String,
    pub auth: FixtureAuth,
    /// Mount a document store, empty at the start of the run.
    #[serde(default)]
    pub docstore: bool,
    /// Mount the event bus and its two endpoints. `None` leaves them unmounted.
    #[serde(default)]
    pub events: Option<FixtureEvents>,
    /// Actions that must be registered, by name.
    #[serde(default)]
    pub actions: Vec<String>,
    /// Mount component federation. `None` leaves it unconfigured.
    #[serde(default)]
    pub components: Option<FixtureComponents>,
    #[serde(default)]
    pub frontend: FixtureFrontend,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureAuth {
    /// Auth on. Off means every endpoint is open and the identity is anonymous.
    pub enabled: bool,
    #[serde(default)]
    pub users: Vec<FixtureUser>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureUser {
    pub name: String,
    /// The plaintext a login sends.
    pub password: String,
    /// How the backend stores the credential, in the `FORGE_AUTH_USERS`
    /// secret syntax: an argon2 PHC hash, or plaintext. Absent means the
    /// password is stored as it stands.
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl FixtureUser {
    /// The stored secret: [`Self::secret`] when authored, otherwise the
    /// password itself.
    pub fn stored_secret(&self) -> &str {
        self.secret.as_deref().unwrap_or(&self.password)
    }
}

/// The event bus and its two endpoints. Both knobs default to the contract's
/// own values; a case that would otherwise wait on one tightens it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureEvents {
    /// Per-subscriber buffer depth. `None` = the backend's default.
    #[serde(default)]
    pub buffer: Option<usize>,
    /// Seconds between server-sent-events heartbeat comments. `None` = the
    /// contract's 15.
    #[serde(default)]
    pub heartbeat_s: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureComponents {
    /// Written to `manifest.json` in the components directory. Absent means
    /// the directory is configured and holds no manifest.
    #[serde(default)]
    pub manifest: Option<Value>,
    /// Written beside it: filename to content.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFrontend {
    /// Written to the static frontend directory: filename to content.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

/// What a case exercises, which decides the shape of its steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Http,
    Sse,
    Ws,
}

/// One contract case.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub kind: Kind,
    /// The fixture this case runs against (`default` unless named).
    #[serde(default = "default_fixture")]
    pub fixture: String,
    /// Why the case is written the way it is. Not asserted on.
    #[serde(default)]
    pub note: Option<String>,
    /// Transports that must run this case.
    pub applies: Vec<String>,
    /// Transports that cannot serve it, and what stops them.
    #[serde(default)]
    pub inapplicable: BTreeMap<String, String>,
    pub steps: Vec<Step>,
}

fn default_fixture() -> String {
    DEFAULT_FIXTURE.to_string()
}

impl Case {
    /// Whether this transport must run the case.
    pub fn applies_to(&self, transport: &str) -> bool {
        self.applies.iter().any(|t| t == transport)
    }
}

/// One move in a case. The variants are distinguished by their key, so a
/// step reads as what it does.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Step {
    /// Send a request and check the response.
    Request {
        request: Request,
        #[serde(default)]
        expect: Option<Expect>,
    },
    /// Open a websocket. `expect` present means the handshake must be refused.
    Connect {
        connect: Connect,
        #[serde(default)]
        expect: Option<Expect>,
    },
    /// Send a JSON frame on the open socket.
    Send { send: Value },
    /// The next frame on the socket must match.
    AwaitFrame { await_frame: Value },
    /// The next event on the stream must match.
    AwaitEvent { await_event: AwaitEvent },
    /// The next block on the stream must be a heartbeat comment, and its text
    /// must match. A heartbeat is not an event, so `await_event` steps over
    /// one and only this step can see it.
    AwaitHeartbeat { await_heartbeat: Value },
}

/// How a request carries its identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    /// No token at all.
    #[default]
    None,
    /// `Authorization: Bearer ${token}`.
    Bearer,
    /// `?token=${token}` — the path `EventSource` and browser sockets need.
    Query,
}

/// One request, as it goes on the wire.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// An HTTP method name, upper-case.
    pub method: String,
    /// A raw URI path, sent verbatim — already percent-encoded where it needs
    /// to be.
    pub path: String,
    /// Query parameters. The driver encodes the values.
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    /// Extra headers, on top of whatever `auth` adds.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub auth: Auth,
    /// A JSON request body, sent with a JSON content type.
    #[serde(default)]
    pub body: Option<Value>,
}

/// A websocket handshake.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connect {
    /// The endpoint to upgrade.
    pub path: String,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    #[serde(default)]
    pub auth: Auth,
}

/// What must come back.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// The HTTP status. A transport without status lines maps it through the
    /// contract's error kinds — see `contract/README.md`.
    pub status: u16,
    /// Header name (lower-case) to a matcher over its value.
    #[serde(default)]
    pub headers: BTreeMap<String, Value>,
    /// Matcher over the parsed JSON body.
    #[serde(default)]
    pub body: Option<Value>,
    /// Matcher over the raw body, for responses that are not JSON.
    #[serde(default)]
    pub text: Option<Value>,
}

/// The next event on a server-sent-events stream.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AwaitEvent {
    /// The topic the event must carry.
    pub topic: String,
    /// Matcher over the event payload.
    pub data: Value,
}

impl Corpus {
    /// Parse and validate the corpus compiled into this crate.
    pub fn load() -> Result<Self, String> {
        Self::parse(CORPUS_JSON)
    }

    /// Parse and validate a corpus from JSON.
    pub fn parse(json: &str) -> Result<Self, String> {
        let corpus: Self =
            serde_json::from_str(json).map_err(|e| format!("corpus is not readable: {e}"))?;
        corpus.validate()?;
        Ok(corpus)
    }

    /// Cases a transport must run, in authored order.
    pub fn cases_for<'a>(&'a self, transport: &'a str) -> impl Iterator<Item = &'a Case> {
        self.cases.iter().filter(move |c| c.applies_to(transport))
    }

    /// The fixture a case runs against. Validation proves every case names one
    /// that exists before anything else asks for it.
    pub fn fixture(&self, case: &Case) -> &Fixture {
        self.fixtures
            .get(&case.fixture)
            .expect("validate() proves every case names a fixture that exists")
    }

    /// The substitution table, ready for a driver to add `token` to.
    pub fn vars(&self) -> Vars {
        self.vars.clone()
    }

    /// Reject a corpus that cannot be run honestly. The rule that matters:
    /// every case accounts for every transport, so a coverage gap has to be
    /// written down rather than left out.
    pub fn validate(&self) -> Result<(), String> {
        if self.transports.is_empty() {
            return Err("corpus declares no transports".into());
        }
        if !self.fixtures.contains_key(DEFAULT_FIXTURE) {
            return Err(format!(
                "corpus declares no {DEFAULT_FIXTURE:?} fixture — it is the one a \
                 case runs against unless it names another"
            ));
        }
        let mut seen_ids = BTreeMap::new();
        let mut used = BTreeMap::new();
        for case in &self.cases {
            if seen_ids.insert(&case.id, ()).is_some() {
                return Err(format!("duplicate case id {:?}", case.id));
            }
            if !self.fixtures.contains_key(&case.fixture) {
                return Err(format!(
                    "case {:?} runs against unknown fixture {:?}",
                    case.id, case.fixture
                ));
            }
            used.insert(case.fixture.clone(), ());
            self.validate_applicability(case)?;
            self.validate_steps(case)?;
        }
        // A fixture no case uses is a server every driver would build for
        // nothing, and reads as coverage that is not there.
        for name in self.fixtures.keys() {
            if !used.contains_key(name) {
                return Err(format!("fixture {name:?} has no case"));
            }
        }
        Ok(())
    }

    fn validate_applicability(&self, case: &Case) -> Result<(), String> {
        let known = |t: &String| self.transports.contains(t);
        for t in &case.applies {
            if !known(t) {
                return Err(format!(
                    "case {:?} applies to unknown transport {t:?}",
                    case.id
                ));
            }
        }
        for (t, reason) in &case.inapplicable {
            if !known(t) {
                return Err(format!(
                    "case {:?} excuses unknown transport {t:?}",
                    case.id
                ));
            }
            if reason.trim().is_empty() {
                return Err(format!("case {:?} excuses {t:?} with no reason", case.id));
            }
            if case.applies_to(t) {
                return Err(format!(
                    "case {:?} both applies to and excuses {t:?}",
                    case.id
                ));
            }
        }
        for t in &self.transports {
            if !case.applies_to(t) && !case.inapplicable.contains_key(t) {
                return Err(format!(
                    "case {:?} says nothing about transport {t:?} — list it under \
                     `applies` or give a reason under `inapplicable`",
                    case.id
                ));
            }
        }
        Ok(())
    }

    fn validate_steps(&self, case: &Case) -> Result<(), String> {
        let Some(first) = case.steps.first() else {
            return Err(format!("case {:?} has no steps", case.id));
        };
        match case.kind {
            Kind::Http => {
                for step in &case.steps {
                    if !matches!(step, Step::Request { .. }) {
                        return Err(format!(
                            "case {:?} is kind `http`, so every step must be a request",
                            case.id
                        ));
                    }
                }
            }
            Kind::Sse | Kind::Ws if self.fixture(case).events.is_none() => {
                return Err(format!(
                    "case {:?} is kind `{:?}`, but its fixture {:?} mounts no event bus",
                    case.id, case.kind, case.fixture
                ))
            }
            Kind::Sse => {
                let Step::Request { expect, .. } = first else {
                    return Err(format!(
                        "case {:?} is kind `sse`, so its first step must be the request \
                         that opens the stream",
                        case.id
                    ));
                };
                // The stream's own response has no body to read — it is the
                // stream. A driver would drop a body expectation authored here
                // without a word, which is the silent gap this corpus exists to
                // stop. Assert on the events instead, with `await_event`.
                if expect
                    .as_ref()
                    .is_some_and(|e| e.body.is_some() || e.text.is_some())
                {
                    return Err(format!(
                        "case {:?} expects a body from the request that opens the stream; \
                         only its status and headers can be checked",
                        case.id
                    ));
                }
                for step in &case.steps {
                    if !matches!(
                        step,
                        Step::Request { .. }
                            | Step::AwaitEvent { .. }
                            | Step::AwaitHeartbeat { .. }
                    ) {
                        return Err(format!(
                            "case {:?} is kind `sse`; a stream cannot be connected to, \
                             sent on, or read a frame from",
                            case.id
                        ));
                    }
                }
            }
            Kind::Ws => {
                if !matches!(first, Step::Connect { .. }) {
                    return Err(format!(
                        "case {:?} is kind `ws`, so its first step must be a connect",
                        case.id
                    ));
                }
                for step in case.steps.iter().skip(1) {
                    if matches!(
                        step,
                        Step::Connect { .. }
                            | Step::AwaitEvent { .. }
                            | Step::AwaitHeartbeat { .. }
                    ) {
                        return Err(format!(
                            "case {:?} connects once, and awaits frames rather than \
                             events or heartbeats",
                            case.id
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A corpus with one fixture and one case, as a string to bend.
    fn minimal() -> String {
        r#"{
            "contract_version": "1.0",
            "transports": ["a", "b"],
            "vars": {},
            "fixtures": {
                "default": {
                    "app": "t",
                    "auth": {"enabled": true, "users": []}
                }
            },
            "cases": [{
                "id": "x",
                "title": "x",
                "applies": ["a", "b"],
                "steps": [{"request": {"method": "GET", "path": "/"},
                           "expect": {"status": 200}}]
            }]
        }"#
        .to_string()
    }

    #[test]
    fn authored_corpus_is_valid() {
        let corpus = Corpus::load().expect("corpus.json");
        assert_eq!(corpus.contract_version, "1.0");
        assert!(corpus.cases_for(RUST_HTTP).count() > 0);
    }

    /// The rule that keeps gaps visible. Drop a transport from a case and the
    /// corpus must refuse to load.
    #[test]
    fn a_case_that_ignores_a_transport_is_rejected() {
        let json = minimal().replace(r#""applies": ["a", "b"]"#, r#""applies": ["a"]"#);
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("says nothing about transport \"b\""), "{err}");
    }

    /// A case runs against `default` unless it names another fixture.
    #[test]
    fn a_case_names_its_fixture_or_gets_the_default() {
        let corpus = Corpus::parse(&minimal()).expect("minimal corpus");
        assert_eq!(corpus.cases[0].fixture, DEFAULT_FIXTURE);

        let json = minimal().replace(r#""id": "x""#, r#""id": "x", "fixture": "nope""#);
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("unknown fixture \"nope\""), "{err}");
    }

    /// A fixture no case uses is a server every driver builds for nothing.
    #[test]
    fn an_unused_fixture_is_rejected() {
        let json = minimal().replace(
            r#""fixtures": {"#,
            r#""fixtures": {"spare": {"app": "t", "auth": {"enabled": false}},"#,
        );
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("fixture \"spare\" has no case"), "{err}");
    }

    /// A stream case on a fixture with no event bus would fail in the driver
    /// with a routing miss, which reads as a contract failure rather than a
    /// corpus one.
    #[test]
    fn a_stream_case_needs_a_fixture_with_an_event_bus() {
        let json = minimal().replace(r#""id": "x""#, r#""id": "x", "kind": "sse""#);
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("mounts no event bus"), "{err}");
    }

    /// A body authored on the stream-opening step would be dropped by the
    /// driver, so the corpus refuses it rather than looking like coverage.
    #[test]
    fn a_stream_cannot_be_asked_for_a_body() {
        let json = CORPUS_JSON.replace(
            "\"headers\": { \"content-type\": { \"$prefix\": \"text/event-stream\" } }",
            "\"headers\": { \"content-type\": { \"$prefix\": \"text/event-stream\" } },\n\
             \"body\": { \"ok\": true }",
        );
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("expects a body from the request"), "{err}");
    }

    /// Authored, not lifted from the real corpus: a test that quotes a reason
    /// verbatim breaks every time someone rewords one.
    #[test]
    fn an_excuse_needs_a_reason() {
        let json = minimal().replace(
            r#""applies": ["a", "b"]"#,
            r#""applies": ["a"], "inapplicable": {"b": "  "}"#,
        );
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("with no reason"), "{err}");
    }

    /// A user's stored secret is the password unless the case authors one —
    /// the hashed credential is the only reason the field exists.
    #[test]
    fn a_user_stores_its_password_unless_a_secret_is_authored() {
        let plain = FixtureUser {
            name: "a".into(),
            password: "pw".into(),
            secret: None,
            roles: vec![],
        };
        assert_eq!(plain.stored_secret(), "pw");
        let hashed = FixtureUser {
            secret: Some("$argon2id$…".into()),
            ..plain
        };
        assert_eq!(hashed.stored_secret(), "$argon2id$…");
    }

    /// The commas in the hash's parameters go into the variable as they are:
    /// reassembling them is the backend's job, and the case that logs that
    /// user in is what checks the backend does it.
    #[test]
    fn the_users_variable_carries_a_hash_verbatim() {
        let auth = FixtureAuth {
            enabled: true,
            users: vec![
                FixtureUser {
                    name: "${user}".into(),
                    password: "pw".into(),
                    secret: None,
                    roles: vec![],
                },
                FixtureUser {
                    name: "hashed".into(),
                    password: "s3cret".into(),
                    secret: Some("$argon2id$v=19$m=19456,t=2,p=1$salt$hash".into()),
                    roles: vec![],
                },
            ],
        };
        let vars = Vars::from([("user".to_string(), "admin".to_string())]);
        assert_eq!(
            users_env(&auth, &vars).expect("users"),
            "admin:pw,hashed:$argon2id$v=19$m=19456,t=2,p=1$salt$hash"
        );
    }
}
