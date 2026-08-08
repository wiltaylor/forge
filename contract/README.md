# The contract corpus

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
| `ts-client` | — | issue #38 |
| `rust-ipc` | — | issue #41 |

The Rust drivers share the loader and the matcher: `crates/forge-contract`.
The Python driver has the same pair in
`python/forge-server/tests/contract/`. Neither knows about HTTP.

`just corpus-test` runs every driver.

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

## Fixture

Every driver provisions the same server before it runs a case. The `fixture`
block says what that is:

- `app` — the application name the backend reports.
- `auth.enabled`, `auth.users` — auth on, with these users. The signing secret
  is the driver's own business; nothing in the corpus observes it.
- `docstore` — a document store, empty at the start of the run.
- `events` — the event bus, with `/api/events` and `/api/ws` mounted.
- `actions` — the actions that must be registered:
  - `echo` returns its payload unchanged.
  - `publish` takes `{topic, data}`, publishes `data` on `topic`, and returns
    `{"published": true, "topic": <topic>}`.
- `components.manifest` — written to `manifest.json` in the components
  directory. `components.files` — written beside it, name to content.
- `frontend.files` — written to the static frontend directory.

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
  "note": "optional; why the case is written this way",
  "applies": [...],
  "inapplicable": {...},
  "steps": [...]
}
```

`kind` is `http` (the default), `sse` or `ws`.

### Steps

A `http` case has only request steps. A `sse` case opens the stream with its
first step. A `ws` case connects with its first step.

| Step | Shape | Meaning |
|---|---|---|
| request | `{"request": {...}, "expect": {...}}` | Send one request, check the response. In a `sse` case the first request step opens the stream: only its status and headers can be checked, and `Corpus::validate` rejects a `body` or `text` authored there rather than letting a driver drop it. |
| connect | `{"connect": {"path": ..., "auth": ...}, "expect": {...}}` | Open a websocket. With no `expect`, the handshake must succeed. With one, it must be refused with that status and body. |
| send | `{"send": {...}}` | Send a JSON frame on the open socket. |
| await_frame | `{"await_frame": <matcher>}` | The **next** frame on the socket must match. |
| await_event | `{"await_event": {"topic": ..., "data": <matcher>}}` | The **next** event on the stream must have this topic and match this data. Heartbeats are not events. |

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
| `{"$exact": <value>}` | Deep equality — no extra keys anywhere. |
| `{"$type": "string"}` | One of `string`, `number`, `integer`, `boolean`, `array`, `object`, `null`. |
| `{"$contains": <matcher>}` | A substring of a string, or an array with at least one matching element. |
| `{"$prefix": "..."}` | A string with this prefix. |
| `{"$min_length": 1}` | A string of at least this many characters. |
| `{"$gt": 0}` | A number greater than this. |

Objects match by subset so that a case can assert the fields it is about and
stay quiet about the rest. Reach for `$exact` when the whole payload is the
point — a document read back, or a websocket frame. Watch the loose ones:
`{"$type": "string"}` accepts `""`, which is why a token asserts
`{"$min_length": 1}` instead.

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
