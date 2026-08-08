# The contract corpus

This directory holds the data that keeps Forge's languages in step.

- `corpus.json` — the API contract, **authored**. Described below.
- `blocks-registry.json`, `emoji.json` — the block kind registry, **generated**
  from Rust. See [the block kind registry](#the-block-kind-registry) at the end.

`corpus.json` is the Forge API contract as authored data. It states what a
request is, what envelope and status come back, and which transports the case
applies to. [`docs/api-contract.md`](../docs/api-contract.md) is the prose; this
file is the table you can run.

A **driver** reads the corpus, builds the fixture, runs every case that applies
to its transport, and fails on the first mismatch. Adding a case here covers
every driver at once.

| Transport id | Driver | Status |
|---|---|---|
| `rust-http` | `crates/forge-server/tests/corpus.rs` | landed |
| `python-http` | `python/forge-server/tests/test_corpus.py` | landed |
| `ts-client` | `packages/client/tests/corpus.test.ts` | landed |
| `rust-ipc` | `crates/forge-tauri/tests/corpus.rs` | landed |

The Rust drivers share the loader and the matcher: `crates/forge-contract`.
The Python driver has the same pair in
`python/forge-server/tests/contract/`, and the TypeScript driver in
`packages/client/tests/contract/`. None of them knows about HTTP.

The TypeScript driver is the one consumer, not a fifth implementation: it
runs every case through the client's own methods against a backend that
`python/forge-server/tests/corpus_fixture_server.py` builds with the Python
driver's harness, and observes the wire through the client's injectable
`fetch`. What it checks on top of the envelope is the client's reading of it —
data unwrapped on success, an `ApiError` carrying the status on failure.

`just corpus-test` runs every driver. `just test` runs all of them except
`rust-ipc`: forge-tauri is its own workspace, because tauri pulls wry/tao/gtk
and would force webkit system deps onto every build. Run `just corpus-test`
(or `just tauri-test`) after touching the corpus or the bridge.

The block editor has a corpus of its own, in the same spirit but over
documents and keypresses rather than requests:
[`contract/blocks/corpus.json`](blocks/README.md), run by
`just block-corpus-test`.

## Applicability, and why it is not optional

Every case names **every** transport, either in `applies` or in
`inapplicable`. `Corpus::validate` rejects a case that leaves one out, so a gap
cannot be created by forgetting. A gap has to be written down, with a reason:

```json
"applies": ["rust-http", "python-http", "ts-client"],
"inapplicable": {
  "rust-ipc": "IPC commands take typed arguments; there is no query string to carry a token."
}
```

A reason states what the transport **cannot** do. "Not implemented yet" is not
a reason — a case that a transport could serve stays in `applies` and fails
until it does. A conditional skip inside a driver is never correct: it turns a
real divergence into a green run.

## Fixtures

A case runs against a server the corpus describes. `fixtures` holds them by
name; a case runs against `default` unless it names another with `"fixture":
"…"`. A driver builds each one it needs and no more, and a fixture no case
uses is rejected — an unused server reads as coverage that is not there.

The four:

| Fixture | Why it exists |
|---|---|
| `default` | Everything mounted, auth on. Nearly every case. |
| `auth-disabled` | The mode the contract calls first-class. The default fixture structurally cannot run it: its cases need a token. |
| `absent-manifest` | A components directory with no `manifest.json`. |
| `events-tuned` | A one-deep buffer and a one-second heartbeat, so the lag notification and the heartbeat are seen rather than waited for. |

A fixture states `app` and `auth.enabled`; everything else defaults to off or
empty, so a fixture carries only what its cases need.

- `app` — the application name the backend reports.
- `auth.enabled`, `auth.users` — auth on, with these users (default none). The
  signing secret is the driver's own business; nothing in the corpus observes
  it.
  - `name`, `password` — the credentials a login sends.
  - `secret` — optional: how the backend **stores** the credential, in the
    `FORGE_AUTH_USERS` syntax (an argon2 PHC hash, or plaintext). Absent means
    the password as it stands.
  - `roles` — optional, default none.
- `docstore` — a document store, empty at the start of the run. Default off.
- `events` — mounts `/api/events` and `/api/ws`. Absent leaves them unmounted.
  `{"buffer": n}` is how far a subscriber may fall behind before it is told it
  lagged, and `{"heartbeat_s": n}` the gap between heartbeat comments; both
  default to the backend's own, which are the contract's.
- `actions` — the actions that must be registered. Default none:
  - `echo` returns its payload unchanged.
  - `publish` takes `{topic, data}`, publishes `data` on `topic`, and returns
    `{"published": true, "topic": <topic>}`.
  - `flood` takes `{topic, count}` and publishes `count` events on `topic`
    without yielding once, so a bounded buffer overruns by construction rather
    than by luck. It returns `{"published": <count>, "topic": <topic>}`.
- `components` — mounts component federation. Absent leaves it unconfigured.
  `components.manifest` is written to `manifest.json`; **absent** is the
  fixture that has a directory and no manifest. `components.files` are written
  beside it, name to content.
- `frontend.files` — written to the static frontend directory. Default none.

### Users go in through the variable

A driver does not hand the user store a name and a secret. It builds the
`FORGE_AUTH_USERS` value the fixture describes — `name:secret` entries, comma
separated — and gives it to the backend's own parser for that variable.

That is the path a deployment takes, and it is the path that has already
broken: an argon2 PHC hash carries commas in its parameters
(`$argon2id$v=19$m=19456,t=2,p=1$…`), which is the separator between entries.
The corpus has a user whose stored secret is a real hash, so a backend that
splits the variable naively boots with a mangled secret and fails
`login-with-a-hashed-credential`.

## Variables

`${name}` in any string — a path, a query value, a header value, a request body
leaf or an expected value — is replaced from `vars`. The driver adds one more:

- `token` — a valid token for `${user}`, obtained by logging in.

## Cases

```json
{
  "id": "unique-kebab-case",
  "title": "one line, present tense",
  "kind": "http",
  "fixture": "default",
  "note": "optional; why the case is written this way",
  "applies": [...],
  "inapplicable": {...},
  "steps": [...]
}
```

`kind` is `http` (the default), `sse` or `ws`. `fixture` names the server the
case runs against, and defaults to `default`.

### Steps

A `http` case has only request steps. A `sse` case opens the stream with its
first step. A `ws` case connects with its first step.

| Step | Shape | Meaning |
|---|---|---|
| request | `{"request": {...}, "expect": {...}}` | Send one request, check the response. In a `sse` case the first request step opens the stream: only its status and headers can be checked, and `Corpus::validate` rejects a `body` or `text` authored there rather than letting a driver drop it. |
| connect | `{"connect": {"path": ..., "auth": ...}, "expect": {...}}` | Open a websocket. With no `expect`, the handshake must succeed. With one, it must be refused with that status and body. |
| send | `{"send": {...}}` | Send a JSON frame on the open socket. |
| await_frame | `{"await_frame": <matcher>}` | The **next** frame on the socket must match. |
| await_event | `{"await_event": {"topic": ..., "data": <matcher>}}` | The **next** event on the stream must have this topic and match this data. Heartbeats are not events, so this steps over one. |
| await_heartbeat | `{"await_heartbeat": <matcher>}` | The **next** block on the stream must be the heartbeat comment, matched against its text without the leading `:`. |

"The next frame" is deliberate. A driver that searched forward for a matching
frame would pass while the server sent frames the contract does not allow.

### Request

```json
{
  "method": "PUT",
  "path": "/api/data/${doc}",
  "query": {"topics": "${sse_topic}"},
  "headers": {"authorization": "Bearer not.a.jwt"},
  "auth": "bearer",
  "body": {"n": 1}
}
```

`path` is a raw URI path and is sent verbatim — a case that needs a space or a
dot segment in a document name authors it percent-encoded, because that is what
the wire carries. `query` values are encoded by the driver.

`auth` is `none` (the default), `bearer` (`Authorization: Bearer ${token}`) or
`query` (`?token=${token}`).

### Expect

```json
{
  "status": 200,
  "headers": {"content-type": {"$prefix": "application/json"}},
  "body": {"ok": true, "data": {...}}
}
```

`status` is the HTTP status. A transport without status lines maps it through
the contract's error kinds, which are one-to-one with the statuses the backends
already use: 400 invalid, 401 unauthorized, 404 not found, 500 internal.

Use `body` for a JSON envelope and `text` for a response that is not JSON. Both
take a matcher.

### Matchers

An expected value is a matcher, not a literal:

| Matcher | Matches |
|---|---|
| `"literal"`, `1`, `true`, `null` | Equality, after `${}` substitution. |
| `{"k": <matcher>}` | An object that has **at least** these keys, each matching. |
| `[<matcher>, ...]` | An array of the same length, element by element. |
| `{"$exact": <matcher>}` | No extra keys, anywhere below this point. Matchers still apply inside it. |
| `{"$type": "string"}` | One of `string`, `number`, `integer`, `boolean`, `array`, `object`, `null`. |
| `{"$contains": <matcher>}` | A substring of a string, or an array with at least one matching element. |
| `{"$prefix": "..."}` | A string with this prefix. |
| `{"$min_length": 1}` | A string of at least this many characters. |
| `{"$gt": 0}` | A number greater than this. |

Objects match by subset so that a case can assert the fields it is about and
stay quiet about the rest. Reach for `$exact` when the whole payload is the
point — a document read back, a websocket frame, or a payload whose *shape* is
what the contract states. `$exact` still runs the matchers inside it, so
`me-with-bearer` can pin four member names while saying only `{"$type":
"integer"}` about the expiry. Watch the loose ones: `{"$type": "string"}`
accepts `""`, which is why a token asserts `{"$min_length": 1}` instead.

An operator object holds exactly one `$` key and nothing else.

## The nineteen cases this replaces

`examples/parity` was nineteen hand-written tests against a live server. Every
one of them is here. The seven parametrised bad names became seven cases, and
two multi-assertion tests split, so the count went up while the coverage stayed
the same.

| `examples/parity` test | Case |
|---|---|
| `token` fixture | `login-returns-token` |
| `test_health_open` | `health-open` |
| `test_login_bad_credentials` | `login-bad-credentials` |
| `test_me_requires_token` | `me-requires-token` |
| `test_me_with_bearer` | `me-with-bearer` |
| `test_query_param_token_accepted` | `me-with-query-token` |
| `test_garbage_token_rejected` | `me-garbage-token-rejected` |
| `test_data_requires_auth` | `data-requires-auth` |
| `test_doc_roundtrip` | `doc-roundtrip` |
| `test_doc_bad_names` | `doc-name-rejected-uppercase`, `-leading-dash`, `-leading-dot`, `-too-long`, `-space`, `-dot-dot`, `-path-separator` |
| `test_action_echo` | `action-echo` |
| `test_action_unknown_404` | `action-unknown` |
| `test_sse_receives_published_event` | `sse-delivers-published-event` |
| `test_api_miss_is_json_404` | `api-miss-is-json-404` |
| `test_spa_fallback_serves_html` | `spa-fallback-serves-html` |
| `test_components_manifest` | `components-manifest`, `component-file-served`, `component-file-traversal-rejected` |
| `test_components_require_auth` | `components-require-auth` |
| `test_ws_requires_token` | `ws-requires-token` |
| `test_ws_ping_pong` | `ws-ping-pong` |
| `test_ws_subscribe_and_receive` | `ws-subscribe-and-receive` |

Five assertions were tightened, because the corpus is authored intent rather
than a description of whatever the server does today:

- The bad-name cases stated `400 or 404 or 405`. Each now states the one status
  its transport must return, and says which rule returns it.
- The traversal guard was checked with `../secret.js`, a path that misses the
  route before the guard sees it. The case now percent-encodes the separator, so
  the guard is what rejects it.
- `test_components_manifest` skipped itself when no components directory was
  configured. The fixture configures one, so there is nothing to skip.
- `test_health_open` asked for a list that holds `echo`. The fixture registers
  exactly `echo` and `publish`, so the case names both, in order.
- The `token` fixture asked for a non-empty string, which `{"$type": "string"}`
  does not say. It asserts `{"$min_length": 1}`.

## The cases the nineteen never reached

Each of these corresponds to something the old suite left unverified, or hid.

| Case | What it reaches |
|---|---|
| `auth-disabled-health-reports-it`, `-has-no-login`, `-identity-is-anonymous`, `-protected-routes-are-open`, `-serves-a-bundle-without-a-token`, `-event-stream-is-open`, `-websocket-is-open` | The mode the contract calls first-class, over every surface: the middleware routes, the bundle endpoint, and the two streams that read their token from a query parameter in their own handlers. The old fixture needed a token, so it could not run the mode that has none. |
| `login-with-a-hashed-credential` | The shipped defect. Both demo configurations ship plaintext credentials, so no suite ever booted a backend with a hash whose parameters carry commas. |
| `components-absent-manifest-is-an-empty-catalogue` | Where the two HTTP backends diverged, and where the suite guarding them skipped itself. |
| `doc-name-rejected-on-read`, `-on-delete` | The name rule on the verbs that are not `PUT`. |
| `doc-name-accepted-at-the-length-limit` | 64 characters, the boundary from the inside. `doc-name-rejected-too-long` moved from 70 characters to 65, so the pair sits either side of it. |
| `component-file-with-query-token` | The `?token=` path on the bundle endpoint, which the contract marks and nothing exercised. |
| `sse-heartbeat-holds-an-idle-stream-open` | The heartbeat comment. It also turned up a difference: sse-starlette's own heartbeat carries a timestamp, and the contract states `: ping`. |
| `ws-lagged-tells-a-consumer-it-missed-events` | The lag notification, which no test had ever provoked. |

`me-with-bearer` was tightened rather than added. It asserted three members
loosely and passed while the two backends answered with different key sets —
one carrying `iat` and dropping a null `iss`, the other the reverse. It now
states the four members the contract names, and nothing else.

That tightening surfaced one more difference: what `exp` means for an
identity that never came from a token. Rust minted a far-future expiry,
Python answered null. Issue #115 settled it as null — there is no expiry
because there was no token — and `auth-disabled-identity-is-anonymous` now
states all four members under `$exact`, the same discipline as
`me-with-bearer`.

## The block kind registry

`blocks-registry.json` and `emoji.json` are **generated**. Do not edit them.

The block kind registry is Rust — `crates/forge-blocks/src/registry.rs`, beside
the schema enum it describes. The two Rust editors read it directly. The web kit
cannot, and `just check` must stay a Node-only job so that installing the web kit
as a git dependency never needs a Rust toolchain. These two files are where the
two halves meet:

```
crates/forge-blocks/src/registry.rs   the registry, authored
  │  just generate-blocks   (cargo; `cargo test -p forge-blocks` fails while stale)
  ▼
contract/blocks-registry.json         the same thing, as data
  │  just generate          (node; `just check` fails while stale)
  ▼
packages/blocks/src/types.gen.ts      the kind union, data-kind list, starters
packages/blocks/src/slash.gen.ts      the slash palette rows
packages/blocks/src/emoji.gen.ts      the emoji table
```

`emoji.json` carries `crates/forge-blocks/src/emoji.rs` the same way: 836
`[shortcode, glyph]` pairs, one line each, in the table's own sorted order.

Neither half can go stale quietly. Each dump records a digest of the Rust file it
came from, so `just check` refuses a dump older than its source and says which
recipe to run — Node compares the digest, and never has to read Rust.
`cargo test -p forge-blocks` fails on the same mismatch from the other side.
