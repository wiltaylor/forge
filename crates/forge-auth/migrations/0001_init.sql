-- Schema rules (sqlx Any driver, shared SQLite/Postgres subset):
-- TEXT for ids/uuids/JSON, BIGINT for unix-second timestamps, INTEGER 0/1 for bools.

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT,
    email_verified INTEGER NOT NULL DEFAULT 0,
    display_name TEXT,
    disabled INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE credentials (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT
);

CREATE TABLE user_roles (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    source TEXT NOT NULL DEFAULT 'manual',
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE signing_keys (
    kid TEXT PRIMARY KEY,
    alg TEXT NOT NULL,
    private_pem TEXT NOT NULL,
    public_pem TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    retired_at BIGINT
);

CREATE TABLE sessions (
    id_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amr TEXT NOT NULL DEFAULT '[]',
    auth_time BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    last_seen BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    revoked_at BIGINT
);
CREATE INDEX idx_sessions_user ON sessions(user_id);

CREATE TABLE clients (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    client_type TEXT NOT NULL DEFAULT 'confidential',
    secret_hash TEXT,
    redirect_uris TEXT NOT NULL DEFAULT '[]',
    post_logout_redirect_uris TEXT NOT NULL DEFAULT '[]',
    allowed_scopes TEXT NOT NULL DEFAULT '["openid","profile","email","roles"]',
    allowed_grants TEXT NOT NULL DEFAULT '["authorization_code","refresh_token"]',
    access_token_ttl BIGINT,
    refresh_token_ttl BIGINT,
    role_mappings TEXT,
    claims_config TEXT,
    exchange_audiences TEXT NOT NULL DEFAULT '[]',
    trusted INTEGER NOT NULL DEFAULT 0,
    legacy_hs256_secret TEXT,
    disabled INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL
);

CREATE TABLE auth_requests (
    id TEXT PRIMARY KEY,
    client_id TEXT,
    params TEXT NOT NULL,
    consented INTEGER NOT NULL DEFAULT 0,
    upstream TEXT,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE TABLE auth_codes (
    code_hash TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT NOT NULL,
    nonce TEXT,
    code_challenge TEXT,
    code_challenge_method TEXT,
    auth_time BIGINT NOT NULL,
    amr TEXT NOT NULL DEFAULT '[]',
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    consumed_at BIGINT
);

CREATE TABLE refresh_tokens (
    id TEXT PRIMARY KEY,
    family_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    scope TEXT NOT NULL,
    parent_id TEXT,
    created_at BIGINT NOT NULL,
    used_at BIGINT,
    expires_at BIGINT NOT NULL,
    revoked_at BIGINT
);
CREATE INDEX idx_refresh_tokens_family ON refresh_tokens(family_id);
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);

CREATE TABLE upstream_providers (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    allow_signup INTEGER NOT NULL DEFAULT 1,
    link_by_email INTEGER NOT NULL DEFAULT 0,
    config TEXT NOT NULL DEFAULT '{}',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE oauth_identities (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES upstream_providers(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject TEXT NOT NULL,
    email TEXT,
    raw_claims TEXT,
    created_at BIGINT NOT NULL,
    UNIQUE (provider_id, subject)
);

CREATE TABLE group_mappings (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES upstream_providers(id) ON DELETE CASCADE,
    external_group TEXT NOT NULL,
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    UNIQUE (provider_id, external_group, role_id)
);
