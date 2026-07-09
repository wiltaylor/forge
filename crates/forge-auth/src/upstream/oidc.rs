//! Generic upstream OIDC connector (covers the Google and Microsoft Entra ID
//! presets — the preset is just a pre-filled issuer URL).

use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde_json::Value;

use super::linking::FederatedProfile;
use crate::db::models::UpstreamProvider;
use crate::error::AppError;
use crate::state::SharedState;

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

fn redirect_url(state: &SharedState, slug: &str) -> String {
    format!("{}/api/callback/{slug}", state.cfg.issuer)
}

async fn build_client(
    state: &SharedState,
    provider: &UpstreamProvider,
) -> Result<CoreClient<
    openidconnect::EndpointSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointMaybeSet,
    openidconnect::EndpointMaybeSet,
>, AppError> {
    let issuer = IssuerUrl::new(cfg_str(provider, "issuer_url")?.to_string())
        .map_err(AppError::internal)?;
    let metadata = CoreProviderMetadata::discover_async(issuer, &state.http)
        .await
        .map_err(|e| AppError::Internal(format!("upstream discovery failed: {e}")))?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg_str(provider, "client_id")?.to_string()),
        Some(ClientSecret::new(cfg_str(provider, "client_secret")?.to_string())),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect_url(state, &provider.slug)).map_err(AppError::internal)?,
    );
    Ok(client)
}

/// Build the upstream authorize URL; returns (url, upstream-state JSON to
/// persist on the auth request).
pub async fn start(
    state: &SharedState,
    provider: &UpstreamProvider,
    request_id: &str,
) -> Result<(String, Value), AppError> {
    let client = build_client(state, provider).await?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let request_id_owned = request_id.to_string();
    let mut auth = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            // `state` round-trips our auth_request id (itself unguessable).
            move || CsrfToken::new(request_id_owned.clone()),
            Nonce::new_random,
        )
        .set_pkce_challenge(pkce_challenge);
    let scopes = provider
        .config
        .get("scopes")
        .and_then(|v| v.as_str())
        .unwrap_or("openid profile email");
    for scope in scopes.split_whitespace() {
        if scope != "openid" {
            auth = auth.add_scope(Scope::new(scope.to_string()));
        }
    }
    let (url, _csrf, nonce) = auth.url();

    let upstream = serde_json::json!({
        "slug": provider.slug,
        "nonce": nonce.secret(),
        "pkce_verifier": pkce_verifier.secret(),
    });
    Ok((url.to_string(), upstream))
}

/// Handle the callback: exchange the code, verify the ID token (signature,
/// issuer, audience, nonce) and map it to a profile.
pub async fn finish(
    state: &SharedState,
    provider: &UpstreamProvider,
    upstream: &Value,
    code: &str,
) -> Result<FederatedProfile, AppError> {
    let client = build_client(state, provider).await?;
    let verifier = upstream
        .get("pkce_verifier")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing pkce state".into()))?;
    let nonce = upstream
        .get("nonce")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing nonce state".into()))?;

    let tokens = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .map_err(AppError::internal)?
        .set_pkce_verifier(PkceCodeVerifier::new(verifier.to_string()))
        .request_async(&state.http)
        .await
        .map_err(|e| AppError::Internal(format!("upstream code exchange failed: {e}")))?;

    let id_token = tokens
        .id_token()
        .ok_or_else(|| AppError::Internal("upstream returned no id_token".into()))?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &Nonce::new(nonce.to_string()))
        .map_err(|e| AppError::Internal(format!("upstream id_token invalid: {e}")))?;

    // The typed claims are verified; re-decode the payload as loose JSON for
    // provider-specific extras (preferred_username, groups claim, ...).
    let raw = decode_jwt_payload(id_token.to_string().as_str());

    let groups = provider
        .config
        .get("groups_claim")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|claim| raw.as_ref()?.get(claim)?.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    Ok(FederatedProfile {
        subject: claims.subject().to_string(),
        email: claims.email().map(|e| e.to_string()),
        email_verified: claims.email_verified().unwrap_or(false),
        display_name: claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.to_string())
            .or_else(|| raw.as_ref()?.get("name")?.as_str().map(String::from)),
        preferred_username: claims
            .preferred_username()
            .map(|u| u.to_string())
            .or_else(|| raw.as_ref()?.get("preferred_username")?.as_str().map(String::from)),
        raw_claims: raw,
        groups,
    })
}

fn decode_jwt_payload(token: &str) -> Option<Value> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
}

/// Admin "test": run discovery and report the token endpoint.
pub async fn test(state: &SharedState, provider: &UpstreamProvider) -> Result<String, AppError> {
    let issuer = IssuerUrl::new(cfg_str(provider, "issuer_url")?.to_string())
        .map_err(AppError::internal)?;
    let metadata = CoreProviderMetadata::discover_async(issuer, &state.http)
        .await
        .map_err(|e| AppError::Internal(format!("discovery failed: {e}")))?;
    Ok(format!(
        "discovery ok, token endpoint: {}",
        metadata
            .token_endpoint()
            .map(|u| u.as_str())
            .unwrap_or("(none)")
    ))
}
