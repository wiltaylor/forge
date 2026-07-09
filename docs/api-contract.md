# Forge API Contract

Version: 1.0 — **frozen**. `@forge/client`, `crates/forge-server` (Rust) and
`python/forge-server` (Python) all conform to this document. Changes require a
version bump and updates to all three implementations plus the parity test
suite (`examples/parity`).

## Envelope

Every JSON response uses one envelope:

- Success: `{"ok": true, "data": <any>}` — mutations may omit `data`.
- Failure: `{"ok": false, "error": "<message>"}` with a meaningful HTTP status.

Wire format is `snake_case` (`expires_at`, `uptime_s`).

## Authentication

- **JWT, HS256 shared secret** (`FORGE_JWT_SECRET`).
- Claims: `sub` (username), `roles` (string array, default `[]`), `iat`,
  `exp` (`iat` + `FORGE_JWT_TTL_SECS`, default 86400), `iss`
  (default `"forge"`; validated only when explicitly configured).
- Transport: `Authorization: Bearer <jwt>` everywhere. A `?token=<jwt>` query
  parameter is **additionally** accepted (header wins) because `EventSource`
  and browser `WebSocket` cannot set headers. Caveat: query tokens can leak
  into access logs — keep TTLs short on exposed deployments.
- **Auth-disabled mode is first-class**: when no `FORGE_JWT_SECRET` is
  configured, every endpoint below is open and handlers see an anonymous
  identity (`sub = "anonymous"`, `roles = []`). A server with a doc store and
  no env vars must run (playpen parity).
- External issuer mode: don't call `/api/auth/login`; share the HS256 secret
  with the issuing service. RS256/JWKS is an extension point
  (`TokenValidator` trait in Rust, validator callable in Python), not v1.

## Endpoints

"Auth: yes" = requires a valid token **when auth is enabled**.

| Endpoint | Method | Auth | Purpose |
|---|---|---|---|
| `/api/health` | GET | no | `{uptime_s, version, app, auth_enabled, actions: [..]}` |
| `/api/auth/login` | POST | no | Body `{username, password}` → `{token, expires_at, user: {name, roles}}`. 401 on bad credentials. 404 when auth is disabled. |
| `/api/auth/me` | GET | yes | Decoded claims `{sub, roles, iss, exp}`. |
| `/api/data` | GET | yes | List docs: `[{name, bytes, modified}]` (`modified` = unix seconds, float). |
| `/api/data/{name}` | GET | yes | Read doc. 404 if missing, 400 on invalid name. |
| `/api/data/{name}` | PUT | yes | Create/replace; body = any JSON; atomic write (tmp + rename). Returns `{ok: true}`. |
| `/api/data/{name}` | DELETE | yes | Idempotent delete. Returns `{ok: true}`. |
| `/api/actions/{name}` | POST | yes | Dispatch a registered action; JSON payload in, JSON out. 404 on unknown action — error message lists registered names. |
| `/api/events` | GET (SSE) | yes (`?token=`) | Optional `?topics=a,b` filter. SSE frames: `event:` = topic, `data:` = JSON. Comment heartbeat `: ping` every 15 s. |
| `/api/ws` | GET upgrade | yes (`?token=`) | JSON frames. Server→client: `{"type": "event", "topic": ..., "data": ...}`, `{"type": "pong"}`, `{"type": "lagged"}`. Client→server: `{"type": "subscribe", "topics": [..]}` (empty/omitted = all), `{"type": "ping"}`. |
| `/api/components` | GET | yes | Federation manifest: `{app, components: [{name, tag, file, hash, props, events, version}]}` (contents of `manifest.json` in the components dir, `app` injected). |
| `/api/components/{file}` | GET | yes (`?token=`) | Serve a bundle file. Filename: `^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$`, no `..`, extension allowlist `.js .mjs .css .map`. |
| `/*` (non-`/api`) | GET | no | Static frontend. Unknown non-`/api` paths fall back to `index.html` (SPA). `/api/*` misses stay JSON 404 envelopes. |

### Document store semantics (playpen lineage)

- Doc name regex: `^[a-z0-9][a-z0-9_-]{0,63}$` — doubles as the
  path-traversal guard. Violations → 400.
- One file per doc: `<data-dir>/<name>.json`.
- Writes are atomic: write `<name>.json.tmp`, then rename over the target.
- DELETE of a missing doc is a success (idempotent).

### Events

- Topics are free-form strings chosen by the app.
- SSE and WS fan out from the same in-process event bus; a slow consumer may
  miss messages (bounded buffers) — WS gets `{"type": "lagged"}`, SSE just
  drops. This is a live-telemetry channel, not a durable queue.

## Environment variables

Both backends load `.env` from the working directory at startup.

| Var | Default | Notes |
|---|---|---|
| `FORGE_JWT_SECRET` | — | Enables auth. Startup fails if set but < 32 chars. |
| `FORGE_AUTH_USERS` | — | Comma-separated entries; **first colon** splits user/secret. Secret starting with `$argon2` is a PHC hash (argon2id verify); anything else is plaintext (startup logs a warning). Example: `admin:admin,ops:$argon2id$...` |
| `FORGE_JWT_TTL_SECS` | `86400` | Token lifetime. |
| `FORGE_JWT_ISS` | `forge` | Issuer claim; validated only if set explicitly. |
| `FORGE_HOST` | `127.0.0.1` | |
| `FORGE_PORT` | `8765` | |
| `FORGE_DATA_DIR` | `./data` | Doc store directory. |
| `FORGE_COMPONENTS_DIR` | `./components` | Federation bundles + `manifest.json`. |
| `FORGE_CORS_ORIGINS` | `http://localhost:5173,http://127.0.0.1:5173` | Must allow the `Authorization` header; never `*` with credentials. Cross-origin only matters for federation fetches and Vite dev. |

Password hash helpers: `forge-hash` (Rust binary, feature `cli`) and
`python -m forge_server.hash`.
