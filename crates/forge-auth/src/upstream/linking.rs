//! Account linking + JIT provisioning for federated logins.
//!
//! Policy: exact `(provider, subject)` link wins → else link by *verified*
//! email when the provider allows it → else JIT-provision when the provider
//! allows signup → else reject.

use crate::db::models::{UpstreamProvider, User};
use crate::db::users::NewUser;
use crate::error::AppError;
use crate::state::SharedState;

pub struct FederatedProfile {
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub preferred_username: Option<String>,
    pub raw_claims: Option<serde_json::Value>,
    /// External group identifiers (claim values or DNs) for role sync.
    pub groups: Option<Vec<String>>,
}

pub async fn resolve_user(
    state: &SharedState,
    provider: &UpstreamProvider,
    profile: &FederatedProfile,
) -> Result<User, AppError> {
    // 1. Existing link.
    if let Some(identity) = state.db.identity_lookup(&provider.id, &profile.subject).await? {
        let user = state
            .db
            .user_by_id(&identity.user_id)
            .await?
            .filter(|u| !u.disabled)
            .ok_or(AppError::Unauthorized)?;
        // Keep email/claims fresh on the identity row.
        state
            .db
            .identity_link(
                &provider.id,
                &user.id,
                &profile.subject,
                profile.email.as_deref(),
                profile.raw_claims.as_ref(),
            )
            .await?;
        sync_groups(state, provider, &user, profile).await?;
        return Ok(user);
    }

    // 2. Link by verified email.
    if provider.link_by_email && profile.email_verified {
        if let Some(email) = &profile.email {
            if let Some(user) = state.db.user_by_verified_email(email).await? {
                state
                    .db
                    .identity_link(
                        &provider.id,
                        &user.id,
                        &profile.subject,
                        profile.email.as_deref(),
                        profile.raw_claims.as_ref(),
                    )
                    .await?;
                sync_groups(state, provider, &user, profile).await?;
                tracing::info!(user = %user.username, provider = %provider.slug, "linked federated identity by email");
                return Ok(user);
            }
        }
    }

    // 3. JIT provisioning.
    if !provider.allow_signup {
        return Err(AppError::Forbidden);
    }
    let username = unique_username(state, provider, profile).await?;
    let user = state
        .db
        .user_create(NewUser {
            username: &username,
            email: profile.email.as_deref(),
            email_verified: profile.email_verified,
            display_name: profile.display_name.as_deref(),
        })
        .await?;
    state
        .db
        .identity_link(
            &provider.id,
            &user.id,
            &profile.subject,
            profile.email.as_deref(),
            profile.raw_claims.as_ref(),
        )
        .await?;
    sync_groups(state, provider, &user, profile).await?;
    tracing::info!(user = %username, provider = %provider.slug, "provisioned user from federated login");
    Ok(user)
}

/// Replace the user's provider-sourced roles from the external group list
/// (only when the provider actually reported groups).
pub async fn sync_groups(
    state: &SharedState,
    provider: &UpstreamProvider,
    user: &User,
    profile: &FederatedProfile,
) -> Result<(), AppError> {
    let Some(groups) = &profile.groups else { return Ok(()) };
    let mappings = state.db.group_mappings_for_provider(&provider.id).await?;
    let role_ids: Vec<String> = mappings
        .iter()
        .filter(|m| groups.iter().any(|g| g.eq_ignore_ascii_case(&m.external_group)))
        .map(|m| m.role_id.clone())
        .collect();
    let source = if provider.kind == "ldap" { "ldap" } else { "federated" };
    state.db.user_roles_replace(&user.id, &role_ids, source).await?;
    Ok(())
}

async fn unique_username(
    state: &SharedState,
    provider: &UpstreamProvider,
    profile: &FederatedProfile,
) -> Result<String, AppError> {
    let base = profile
        .preferred_username
        .clone()
        .or_else(|| {
            profile
                .email
                .as_deref()
                .and_then(|e| e.split('@').next())
                .map(String::from)
        })
        .unwrap_or_else(|| format!("{}-{}", provider.slug, &profile.subject.chars().take(8).collect::<String>()));
    let base: String = base
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' { c } else { '-' })
        .collect();

    if state.db.user_by_username(&base).await?.is_none() {
        return Ok(base);
    }
    for n in 2..100 {
        let candidate = format!("{base}{n}");
        if state.db.user_by_username(&candidate).await?.is_none() {
            return Ok(candidate);
        }
    }
    Err(AppError::Conflict("could not derive a unique username".into()))
}
