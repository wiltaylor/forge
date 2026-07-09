# forge-auth

Self-hosted OIDC identity provider for the Forge ecosystem: one login for all
your forge apps (and anything else that speaks OpenID Connect).

- **Rust** backend on `forge-server` (axum), **SolidJS** frontend on `@forge/ui`,
  shipped as a single binary with the SPA embedded.
- **Full OIDC provider**: discovery, authorization code + PKCE, JWKS (RS256),
  userinfo, revocation (RFC 7009), introspection (RFC 7662), RP-initiated logout.
- **RFC 8693 token exchange**: swap one login for app-scoped tokens per client
  (`aud`), with per-client role mapping/claim selection — plus a legacy mode
  that mints forge-server-style HS256 tokens for unmodified forge apps.
- **Login methods**: username/password (argon2id), upstream OAuth/OIDC
  federation (generic OIDC + Google / Microsoft Entra ID / GitHub presets),
  and LDAP bind with Active Directory defaults + `memberOf` group→role sync.
- **Member database** on SQLite *or* Postgres (`DATABASE_URL`), storing users
  and roles; roles are encoded into JWT claims.
- **Refresh tokens** with rotation and reuse detection (family revocation).
- **Admin console**: users, roles, clients, providers, sessions, key rotation.

## Quick start

All recipes run from the repo root:

```sh
just auth-build          # frontend + release binary (SPA embedded)
just auth-dev            # debug server on :8770 with dev-login compiled in
just auth-frontend-dev   # vite on :5174 (hot reload, proxies to :8770)
```

First boot creates an `admin` user (password from `FORGE_AUTH_ADMIN_PASSWORD`,
or generated and logged once). Copy `crates/forge-auth/.env.example` to `.env`
at the repo root for local dev.

## Development login mode

The `dev-login` cargo feature compiles in a password-less "pick a user" panel
(dev-admin / dev-user / dev-viewer) on the login page. It is a **compile-time**
feature — a production binary contains none of it (`/api/login/dev/*` does not
exist). `just auth-dev` and `just auth-docker-build-dev` (tag
`forge-auth:dev`) are the only ways to get it.

## Docker

```sh
just auth-docker-build        # production image  forge-auth:latest
just auth-docker-build-dev    # DEV image         forge-auth:dev (dev-login!)
just auth-docker-up           # compose stack (SQLite volume; Postgres variant inside)
```

The image builds from the repo root as a single context — no external build
context needed. Set `FORGE_AUTH_ISSUER` to the public URL
(e.g. `https://auth.example.lan`) — everything derives from it.

## Connecting a forge app (or any OIDC RP)

1. Admin console → Clients → New client. Register the redirect URI
   (e.g. `https://app.example.lan/cb`). Copy the client secret (shown once).
2. Point the app at the discovery document:
   `https://auth.example.lan/.well-known/openid-configuration`.
3. Roles arrive in the `roles` claim of the access/ID token; use per-client
   *role mappings* to rename/filter them.
4. For service-to-service fan-out, enable *token exchange audiences* on the
   requesting client and POST `grant_type=urn:ietf:params:oauth:grant-type:token-exchange`
   with `audience=<target client_id>`.
5. Legacy forge apps that only know `FORGE_JWT_SECRET`: set the client's
   *legacy HS256 secret* to that value and exchange tokens for it.

`examples/auth-demo` is a minimal relying party for manual testing
(`cargo run -p auth-demo`).

## Testing

```sh
cargo test -p forge-auth   # rust unit + integration tests
just auth-test-devlogin    # dev-login gating tests (feature compiled in)
just auth-e2e-test         # end-to-end over real HTTP, incl. federation between two instances
```

## Layout

```
crates/forge-auth/           Rust IdP (axum on forge-server; sqlx Any → sqlite/postgres)
apps/auth/                   SolidJS SPA: hosted login/consent + admin console (@forge/ui)
scripts/oidc_flow_test.py    uv-run e2e test
examples/auth-demo/          minimal relying party
```
