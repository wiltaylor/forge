//! The Rust IPC driver for the contract corpus (`contract/corpus.json`).
//!
//! It builds the fixture the corpus describes, then runs every case that
//! declares `rust-ipc` under `applies`. The case list lives in the corpus, not
//! here — this file only knows how to turn an authored request into an IPC
//! request and hand the response back to the matcher.
//!
//! There is no Tauri runtime in the loop: `Builder::try_state` assembles the
//! same plugin state the `setup` hook does, and `ForgeState::request` is the
//! same entry point the `plugin:forge|request` command calls. What a real app
//! adds on top is the invoke boundary, which carries the arguments verbatim.
//!
//! A case this transport cannot serve is declared inapplicable in the corpus,
//! with a reason. This driver has no skips: an authored expectation it cannot
//! check is a failure, because a silent pass is what the corpus exists to stop.

use std::collections::BTreeMap;

use forge_contract::{
    interpolate, interpolate_value, match_value, users_env, write_fixture_files, Auth, Case,
    Corpus, Expect, Fixture, Kind, Step, Vars, RUST_IPC,
};
use forge_tauri::{ActionCtx, AuthConfig, Builder, ForgeResponse, ForgeState};
use serde_json::{json, Value};

/// Driver-local: the corpus does not observe the signing secret.
const SECRET: &str = "0123456789abcdef0123456789abcdef";

#[tokio::test]
async fn corpus_rust_ipc() {
    let corpus = Corpus::load().expect("contract/corpus.json");

    // One state per fixture a case actually names, built the first time it is
    // needed: a fixture whose cases are all inapplicable here costs nothing.
    let mut harnesses: BTreeMap<&str, Harness> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut ran = 0;
    for case in corpus.cases_for(RUST_IPC) {
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

    assert!(ran > 0, "no corpus case applies to {RUST_IPC}");
    assert!(
        failures.is_empty(),
        "{} of {ran} contract cases failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// The fixture the corpus describes, plus the token every case borrows.
struct Harness {
    state: ForgeState,
    vars: Vars,
    /// Kept alive: the doc store and the components directory live under it.
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
        for path in [&data, &components] {
            std::fs::create_dir_all(path).expect("fixture dir");
        }
        // `fixture.frontend` has no counterpart here: the webview loads its
        // own assets, which is why the static cases are inapplicable.

        let mut builder = Builder::new(interpolate(&fixture.app, &vars).expect("app"));
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
            builder = builder.with_components(&components);
        }
        if fixture.docstore {
            builder = builder.with_docstore(&data);
        }
        if fixture.auth.enabled {
            builder = builder.auth(auth_config(fixture, &vars));
        }
        for name in &fixture.actions {
            builder = register_action(builder, name);
        }
        // `fixture.events` needs no wiring: the bus is always live and the
        // plugin fans it out as Tauri events, which no case observes.
        let state = builder.try_state().expect("fixture state");

        let mut harness = Self {
            state,
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
        let res = self
            .state
            .request("POST", "/api/auth/login", Some(body), None)
            .await;
        assert_eq!(res.status, 200, "fixture login: {}", res.body);
        res.body
            .pointer("/data/token")
            .and_then(Value::as_str)
            .expect("login returns a token")
            .to_string()
    }

    async fn run(&self, case: &Case) -> Result<(), String> {
        if case.kind != Kind::Http {
            return Err(format!(
                "kind `{:?}` needs a stream this transport does not have; \
                 such a case belongs under `inapplicable`",
                case.kind
            ));
        }
        for (i, step) in case.steps.iter().enumerate() {
            let Step::Request { request, expect } = step else {
                return Err(format!("step {i}: kind `http` takes request steps only"));
            };
            let res = self.send(request, i).await?;
            self.check(expect.as_ref(), &res, i)?;
        }
        Ok(())
    }

    /// One authored request, as an IPC request. The path travels verbatim —
    /// the bridge percent-decodes each segment the way axum decodes a path
    /// parameter — and the token travels as an argument.
    async fn send(
        &self,
        request: &forge_contract::Request,
        step: usize,
    ) -> Result<ForgeResponse, String> {
        let at = |e: String| format!("step {step}: {e}");
        if !request.query.is_empty() {
            return Err(at("an IPC request carries no query string, so it cannot \
                           carry a query parameter; a case that needs one belongs \
                           under `inapplicable`"
                .into()));
        }
        let path = interpolate(&request.path, &self.vars).map_err(at)?;
        let body = match &request.body {
            Some(body) => Some(interpolate_value(body, &self.vars).map_err(at)?),
            None => None,
        };
        let token = self.token(request).map_err(at)?;
        Ok(self
            .state
            .request(&request.method, &path, body, token.as_deref())
            .await)
    }

    /// The token this request carries. `bearer` takes the fixture's; an
    /// authored `authorization` header is the case that hands over a token of
    /// its own, so its bearer value is unwrapped into the argument.
    fn token(&self, request: &forge_contract::Request) -> Result<Option<String>, String> {
        let mut token = match request.auth {
            Auth::None => None,
            Auth::Bearer => Some(self.var("token")?),
            Auth::Query => {
                return Err("an IPC request carries no query string to hold a token; \
                            the token is an argument, and `auth: bearer` is how a \
                            case asks for it"
                    .into())
            }
        };
        for (name, value) in &request.headers {
            if !name.eq_ignore_ascii_case("authorization") {
                return Err(format!(
                    "an IPC request carries no {name:?} header; a case that needs \
                     one belongs under `inapplicable`"
                ));
            }
            let value = interpolate(value, &self.vars)?;
            let bearer = value.strip_prefix("Bearer ").ok_or_else(|| {
                format!("only a Bearer authorization header maps onto IPC, got {value:?}")
            })?;
            token = Some(bearer.trim().to_string());
        }
        Ok(token)
    }

    fn check(
        &self,
        expect: Option<&Expect>,
        res: &ForgeResponse,
        step: usize,
    ) -> Result<(), String> {
        let Some(expect) = expect else { return Ok(()) };
        let at = |e: String| format!("step {step}: {e}");
        if res.status != expect.status {
            return Err(at(format!(
                "expected status {}, got {} ({})",
                expect.status, res.status, res.body
            )));
        }
        if !expect.headers.is_empty() {
            return Err(at("an IPC response carries no headers; a case that \
                            asserts one belongs under `inapplicable`"
                .into()));
        }
        if expect.text.is_some() {
            return Err(at("an IPC response carries a JSON envelope, never raw \
                            text; a case that asserts text belongs under \
                            `inapplicable`"
                .into()));
        }
        if let Some(want) = &expect.body {
            match_value(want, &res.body, &self.vars).map_err(at)?;
        }
        Ok(())
    }

    fn var(&self, name: &str) -> Result<String, String> {
        self.vars
            .get(name)
            .cloned()
            .ok_or_else(|| format!("no ${{{name}}} in the substitution table"))
    }
}

/// The fixture's users, configured the way a deployment configures them:
/// through the `FORGE_AUTH_USERS` parser. Handing the store a name and a
/// secret directly would step over the parse, which is where an argon2 hash's
/// commas land.
fn auth_config(fixture: &Fixture, vars: &Vars) -> AuthConfig {
    let raw = users_env(&fixture.auth, vars).expect("fixture users");
    let mut config = AuthConfig::new(SECRET);
    config.users = forge_core::auth::parse_users(&raw)
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

/// The behaviour each fixture action must have, per `contract/README.md`. An
/// action the corpus names and this driver does not know is a failure, not a
/// silent gap.
fn register_action(builder: Builder, name: &str) -> Builder {
    match name {
        "echo" => builder.action("echo", |payload, _ctx| async move { Ok(payload) }),
        "publish" => builder.action("publish", |payload: Value, ctx: ActionCtx| async move {
            let topic = payload
                .get("topic")
                .and_then(Value::as_str)
                .unwrap_or("misc")
                .to_string();
            let data = payload.get("data").cloned().unwrap_or(Value::Null);
            ctx.events.publish(&topic, data);
            Ok(json!({"published": true, "topic": topic}))
        }),
        other => {
            panic!("the corpus fixture wants an action this driver has no behaviour for: {other:?}")
        }
    }
}
