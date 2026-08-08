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
    /// The server every driver must build before running a case.
    pub fixture: Fixture,
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
    pub docstore: bool,
    /// Mount the event bus and its two endpoints.
    pub events: bool,
    /// Actions that must be registered, by name.
    pub actions: Vec<String>,
    pub components: FixtureComponents,
    pub frontend: FixtureFrontend,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureAuth {
    /// Auth on. Off means every endpoint is open and the identity is anonymous.
    pub enabled: bool,
    pub users: Vec<FixtureUser>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureUser {
    pub name: String,
    /// Plaintext. How a driver stores it is its own business.
    pub password: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureComponents {
    /// Written to `manifest.json` in the components directory.
    pub manifest: Value,
    /// Written beside it: filename to content.
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFrontend {
    /// Written to the static frontend directory: filename to content.
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
        let mut seen_ids = BTreeMap::new();
        for case in &self.cases {
            if seen_ids.insert(&case.id, ()).is_some() {
                return Err(format!("duplicate case id {:?}", case.id));
            }
            self.validate_applicability(case)?;
            self.validate_steps(case)?;
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
                    if matches!(step, Step::Connect { .. } | Step::Send { .. }) {
                        return Err(format!(
                            "case {:?} is kind `sse`; a stream cannot be connected to or \
                             sent on",
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
                    if matches!(step, Step::Connect { .. } | Step::AwaitEvent { .. }) {
                        return Err(format!(
                            "case {:?} connects once, and awaits frames rather than events",
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
        let json = r#"{
            "contract_version": "1.0",
            "transports": ["a", "b"],
            "vars": {},
            "fixture": {
                "app": "t",
                "auth": {"enabled": true, "users": []},
                "docstore": true,
                "events": true,
                "actions": [],
                "components": {"manifest": {}, "files": {}},
                "frontend": {"files": {}}
            },
            "cases": [{
                "id": "x",
                "title": "x",
                "applies": ["a"],
                "steps": [{"request": {"method": "GET", "path": "/"},
                           "expect": {"status": 200}}]
            }]
        }"#;
        let err = Corpus::parse(json).unwrap_err();
        assert!(err.contains("says nothing about transport \"b\""), "{err}");
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

    #[test]
    fn an_excuse_needs_a_reason() {
        let json = CORPUS_JSON.replace(
            "\"rust-ipc\": \"IPC has no URL space, so there is no unknown path to miss.\"",
            "\"rust-ipc\": \"  \"",
        );
        let err = Corpus::parse(&json).unwrap_err();
        assert!(err.contains("with no reason"), "{err}");
    }
}
