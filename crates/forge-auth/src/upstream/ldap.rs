//! LDAP bind authentication with Active Directory-friendly defaults.
//!
//! Flow: service-account bind → search for the user (filter has `{username}`
//! substituted, escaped) → bind as the found DN with the submitted password →
//! sync `memberOf` groups through the provider's group→role mappings.

use ldap3::{ldap_escape, LdapConnAsync, LdapConnSettings, Scope, SearchEntry};

use super::linking::{self, FederatedProfile};
use crate::db::models::{UpstreamProvider, User};
use crate::error::AppError;
use crate::state::SharedState;

const DEFAULT_FILTER: &str = "(&(objectClass=user)(sAMAccountName={username}))";

struct LdapConfig {
    url: String,
    starttls: bool,
    bind_dn: String,
    bind_password: String,
    base_dn: String,
    user_filter: String,
    email_attr: String,
    display_name_attr: String,
}

fn parse_config(provider: &UpstreamProvider) -> Result<LdapConfig, AppError> {
    let get = |key: &str| provider.config.get(key).and_then(|v| v.as_str()).unwrap_or("");
    let required = |key: &str| -> Result<String, AppError> {
        let v = get(key);
        if v.is_empty() {
            return Err(AppError::Config(format!(
                "ldap provider {:?} is missing config key {key:?}",
                provider.slug
            )));
        }
        Ok(v.to_string())
    };
    Ok(LdapConfig {
        url: required("url")?,
        starttls: provider.config.get("starttls").and_then(|v| v.as_bool()).unwrap_or(false),
        bind_dn: required("bind_dn")?,
        bind_password: required("bind_password")?,
        base_dn: required("base_dn")?,
        user_filter: {
            let f = get("user_filter");
            if f.is_empty() { DEFAULT_FILTER.to_string() } else { f.to_string() }
        },
        email_attr: {
            let a = get("email_attr");
            if a.is_empty() { "mail".to_string() } else { a.to_string() }
        },
        display_name_attr: {
            let a = get("display_name_attr");
            if a.is_empty() { "displayName".to_string() } else { a.to_string() }
        },
    })
}

async fn connect(config: &LdapConfig) -> Result<ldap3::Ldap, AppError> {
    let settings = LdapConnSettings::new().set_starttls(config.starttls);
    let (conn, ldap) = LdapConnAsync::with_settings(settings, &config.url)
        .await
        .map_err(|e| AppError::Internal(format!("ldap connect failed: {e}")))?;
    ldap3::drive!(conn);
    Ok(ldap)
}

/// Try every enabled LDAP provider for a username/password pair. Returns the
/// (possibly just-provisioned) local user on success, `None` when no LDAP
/// provider claims the login.
pub async fn try_ldap_login(
    state: &SharedState,
    username: &str,
    password: &str,
) -> Result<Option<User>, AppError> {
    // An empty password would turn a user bind into an anonymous bind that
    // "succeeds" on many servers — never send one.
    if password.is_empty() || username.is_empty() {
        return Ok(None);
    }
    let providers = state.db.providers_enabled().await?;
    for provider in providers.iter().filter(|p| p.kind == "ldap") {
        match bind_and_provision(state, provider, username, password).await {
            Ok(Some(user)) => return Ok(Some(user)),
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!(provider = %provider.slug, error = %e, "ldap login attempt failed");
                continue;
            }
        }
    }
    Ok(None)
}

async fn bind_and_provision(
    state: &SharedState,
    provider: &UpstreamProvider,
    username: &str,
    password: &str,
) -> Result<Option<User>, AppError> {
    let config = parse_config(provider)?;
    let mut ldap = connect(&config).await?;

    let result = lookup_and_bind(&mut ldap, &config, username, password).await;
    let _ = ldap.unbind().await;
    let Some(entry) = result? else { return Ok(None) };

    let email = entry.attrs.get(&config.email_attr).and_then(|v| v.first()).cloned();
    let display_name = entry
        .attrs
        .get(&config.display_name_attr)
        .and_then(|v| v.first())
        .cloned();
    let groups = entry.attrs.get("memberOf").cloned().unwrap_or_default();

    let profile = FederatedProfile {
        subject: entry.dn.clone(),
        email,
        // Directory-sourced addresses are as verified as we can get.
        email_verified: true,
        display_name,
        preferred_username: Some(username.to_string()),
        raw_claims: None,
        groups: Some(groups),
    };
    let user = match linking::resolve_user(state, provider, &profile).await {
        Ok(user) => user,
        Err(AppError::Forbidden) => {
            tracing::info!(provider = %provider.slug, username, "ldap bind ok but signup disabled and no link");
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    Ok(Some(user))
}

async fn lookup_and_bind(
    ldap: &mut ldap3::Ldap,
    config: &LdapConfig,
    username: &str,
    password: &str,
) -> Result<Option<SearchEntry>, AppError> {
    ldap.simple_bind(&config.bind_dn, &config.bind_password)
        .await
        .map_err(|e| AppError::Internal(format!("ldap service bind failed: {e}")))?
        .success()
        .map_err(|e| AppError::Internal(format!("ldap service bind rejected: {e}")))?;

    let filter = config
        .user_filter
        .replace("{username}", &ldap_escape(username));
    let (results, _) = ldap
        .search(
            &config.base_dn,
            Scope::Subtree,
            &filter,
            vec!["dn", &config.email_attr, &config.display_name_attr, "memberOf"],
        )
        .await
        .map_err(|e| AppError::Internal(format!("ldap search failed: {e}")))?
        .success()
        .map_err(|e| AppError::Internal(format!("ldap search rejected: {e}")))?;

    let Some(first) = results.into_iter().next() else { return Ok(None) };
    let entry = SearchEntry::construct(first);

    // The actual authentication: bind as the user.
    let bound = ldap
        .simple_bind(&entry.dn, password)
        .await
        .map_err(|e| AppError::Internal(format!("ldap user bind failed: {e}")))?;
    if bound.success().is_err() {
        return Ok(None);
    }
    Ok(Some(entry))
}

/// Admin "test": connect + service bind + base search.
pub async fn test(_state: &SharedState, provider: &UpstreamProvider) -> Result<String, AppError> {
    let config = parse_config(provider)?;
    let mut ldap = connect(&config).await?;
    let outcome = async {
        ldap.simple_bind(&config.bind_dn, &config.bind_password)
            .await
            .map_err(|e| AppError::Internal(format!("connect ok, service bind failed: {e}")))?
            .success()
            .map_err(|e| AppError::Internal(format!("connect ok, service bind rejected: {e}")))?;
        ldap.search(&config.base_dn, Scope::Base, "(objectClass=*)", vec!["dn"])
            .await
            .map_err(|e| AppError::Internal(format!("bind ok, base search failed: {e}")))?
            .success()
            .map_err(|e| AppError::Internal(format!("bind ok, base search rejected: {e}")))?;
        Ok::<_, AppError>(())
    }
    .await;
    let _ = ldap.unbind().await;
    outcome?;
    Ok(format!("connected to {}, service bind + base search ok", config.url))
}
