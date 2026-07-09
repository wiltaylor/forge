//! GitHub OAuth2 connector (GitHub has no OIDC, so we fetch the profile from
//! the REST API after the code exchange).

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse,
    TokenUrl,
};
use serde_json::Value;

use super::linking::FederatedProfile;
use crate::db::models::UpstreamProvider;
use crate::error::AppError;
use crate::state::SharedState;

const AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API: &str = "https://api.github.com";

fn cfg_str<'a>(provider: &'a UpstreamProvider, key: &str) -> Result<&'a str, AppError> {
    provider
        .config
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::Config(format!("provider {:?} is missing config key {key:?}", provider.slug))
        })
}

fn build_client(
    state: &SharedState,
    provider: &UpstreamProvider,
) -> Result<
    BasicClient<
        oauth2::EndpointSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointSet,
    >,
    AppError,
> {
    Ok(BasicClient::new(ClientId::new(cfg_str(provider, "client_id")?.to_string()))
        .set_client_secret(ClientSecret::new(cfg_str(provider, "client_secret")?.to_string()))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string()).map_err(AppError::internal)?)
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string()).map_err(AppError::internal)?)
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/api/callback/{}", state.cfg.issuer, provider.slug))
                .map_err(AppError::internal)?,
        ))
}

pub fn start(
    state: &SharedState,
    provider: &UpstreamProvider,
    request_id: &str,
) -> Result<(String, Value), AppError> {
    let client = build_client(state, provider)?;
    let request_id_owned = request_id.to_string();
    let (url, _csrf) = client
        .authorize_url(move || CsrfToken::new(request_id_owned.clone()))
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .url();
    Ok((url.to_string(), serde_json::json!({ "slug": provider.slug })))
}

pub async fn finish(
    state: &SharedState,
    provider: &UpstreamProvider,
    code: &str,
) -> Result<FederatedProfile, AppError> {
    let client = build_client(state, provider)?;
    let tokens = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .request_async(&state.http)
        .await
        .map_err(|e| AppError::Internal(format!("github code exchange failed: {e}")))?;
    let token = tokens.access_token().secret();

    let user: Value = api_get(state, token, "/user").await?;
    let subject = user
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::Internal("github /user returned no id".into()))?
        .to_string();

    // Primary verified email (the profile "email" field is often null).
    let emails: Value = api_get(state, token, "/user/emails").await.unwrap_or(Value::Null);
    let primary = emails.as_array().and_then(|list| {
        list.iter()
            .find(|e| e["primary"] == true && e["verified"] == true)
            .or_else(|| list.iter().find(|e| e["verified"] == true))
    });

    Ok(FederatedProfile {
        subject,
        email: primary
            .and_then(|e| e["email"].as_str().map(String::from))
            .or_else(|| user["email"].as_str().map(String::from)),
        email_verified: primary.is_some(),
        display_name: user["name"].as_str().map(String::from),
        preferred_username: user["login"].as_str().map(String::from),
        raw_claims: Some(user),
        groups: None,
    })
}

async fn api_get(state: &SharedState, token: &str, path: &str) -> Result<Value, AppError> {
    let res = state
        .http
        .get(format!("{API}{path}"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("github api call failed: {e}")))?;
    if !res.status().is_success() {
        return Err(AppError::Internal(format!("github api {path} returned {}", res.status())));
    }
    res.json().await.map_err(AppError::internal)
}

pub fn test(provider: &UpstreamProvider) -> Result<String, AppError> {
    cfg_str(provider, "client_id")?;
    cfg_str(provider, "client_secret")?;
    Ok("client id/secret configured (GitHub has no discovery to probe)".into())
}
