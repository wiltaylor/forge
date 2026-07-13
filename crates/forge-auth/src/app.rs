//! Route table and server assembly on top of `ForgeApp`.

use axum::routing::{delete, get, post, put};
use axum::{Extension, Router};
use forge_server::ForgeApp;

use crate::error::AppError;
use crate::state::SharedState;
use crate::{admin, login, me, oidc};

/// All IdP routes are registered via `ForgeApp::route` (outside forge's auth
/// middleware — each handler brings its own auth: session cookie, client
/// authentication, or Bearer token).
pub fn build_router(state: SharedState, forge_app: ForgeApp) -> Result<Router, AppError> {
    let app = add_dev_routes(forge_app)
        // --- OIDC protocol surface (raw spec JSON) ---
        .route(
            "/.well-known/openid-configuration",
            get(oidc::discovery::openid_configuration),
        )
        .route("/.well-known/jwks.json", get(oidc::discovery::jwks))
        .route("/oauth2/authorize", get(oidc::authorize::authorize))
        .route("/oauth2/token", post(oidc::token::token))
        .route(
            "/oauth2/userinfo",
            get(oidc::userinfo::userinfo).post(oidc::userinfo::userinfo),
        )
        .route("/oauth2/revoke", post(oidc::revoke::revoke))
        .route("/oauth2/introspect", post(oidc::introspect::introspect))
        .route(
            "/oauth2/logout",
            get(oidc::end_session::end_session).post(oidc::end_session::end_session),
        )
        // --- browser/SPA API (forge envelope) ---
        .route("/api/login", post(login::login))
        .route("/api/login/providers", get(login::login_providers))
        .route("/api/login/request/{id}", get(login::login_request_info))
        .route(
            "/api/login/upstream/{slug}",
            get(crate::upstream::start_upstream_login),
        )
        .route(
            "/api/callback/{slug}",
            get(crate::upstream::upstream_callback),
        )
        .route("/api/session", get(login::session_info))
        .route("/api/logout", post(login::logout))
        .route(
            "/api/consent/{id}",
            get(oidc::authorize::consent_info).post(oidc::authorize::consent_decide),
        )
        // --- self-service ---
        .route("/api/me", get(me::profile))
        .route("/api/me/password", post(me::change_password))
        .route("/api/me/sessions", get(me::sessions))
        .route("/api/me/sessions/{id}", delete(me::revoke_session))
        .route("/api/me/identities/{id}", delete(me::unlink_identity))
        // --- admin ---
        .route(
            "/api/admin/users",
            get(admin::users::list).post(admin::users::create),
        )
        .route(
            "/api/admin/users/{id}",
            get(admin::users::get)
                .put(admin::users::update)
                .delete(admin::users::delete),
        )
        .route(
            "/api/admin/users/{id}/password",
            put(admin::users::set_password),
        )
        .route(
            "/api/admin/users/{id}/roles",
            put(admin::users::set_user_roles),
        )
        .route(
            "/api/admin/roles",
            get(admin::roles::list).post(admin::roles::create),
        )
        .route("/api/admin/roles/{id}", delete(admin::roles::delete))
        .route(
            "/api/admin/clients",
            get(admin::clients::list).post(admin::clients::create),
        )
        .route(
            "/api/admin/clients/{id}",
            get(admin::clients::get)
                .put(admin::clients::update)
                .delete(admin::clients::delete),
        )
        .route(
            "/api/admin/clients/{id}/secret",
            post(admin::clients::regenerate_secret),
        )
        .route(
            "/api/admin/providers",
            get(admin::providers::list).post(admin::providers::upsert),
        )
        .route(
            "/api/admin/providers/{id}",
            get(admin::providers::get).delete(admin::providers::delete),
        )
        .route(
            "/api/admin/providers/{id}/test",
            post(admin::providers::test),
        )
        .route("/api/admin/sessions", get(admin::sessions::list))
        .route("/api/admin/sessions/{id}", delete(admin::sessions::revoke))
        .route("/api/admin/keys", get(admin::keys::list))
        .route("/api/admin/keys/rotate", post(admin::keys::rotate));

    let router = app
        .try_router()
        .map_err(|e| AppError::Config(e.to_string()))?
        .layer(Extension(state));
    Ok(router)
}

/// Dev-login routes exist only when the feature is compiled in.
#[cfg(feature = "dev-login")]
pub fn add_dev_routes(app: ForgeApp) -> ForgeApp {
    app.route("/api/login/dev/users", get(login::dev::list_users))
        .route("/api/login/dev", post(login::dev::login))
}

#[cfg(not(feature = "dev-login"))]
pub fn add_dev_routes(app: ForgeApp) -> ForgeApp {
    app
}
