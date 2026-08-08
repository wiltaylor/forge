//! HTTP authentication: the request extractor, and the login/me routes that
//! wrap it.
//!
//! The auth behaviour itself — token validation, the login user store,
//! password verification — lives in [`forge_core::auth`], so a non-HTTP
//! transport can authenticate with the same code. What is left here is how a
//! token is pulled out of an HTTP request.

pub mod extract;
pub mod routes;

// Re-exported so the `forge_server::auth::*` paths downstream apps already
// use keep working now that the implementation sits in forge-core.
pub use forge_core::auth::{
    decode_token, encode_token, parse_users, unix_now, Auth, AuthConfig, AuthUser, Claims,
    Hs256Validator, LoginResponse, TokenValidator, AUTH_DISABLED, DEFAULT_ISS, DEFAULT_TTL_SECS,
};
