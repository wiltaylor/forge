//! `FORGE_AUTH_USERS` parsing and secret verification.
//!
//! Format: comma-separated entries; the FIRST colon splits user from secret.
//! A secret starting with `$argon2` is a PHC hash verified with argon2;
//! anything else is a plaintext password (a warning is logged).

use argon2::password_hash::PasswordHash;
use argon2::{Argon2, PasswordVerifier};

use crate::error::ForgeError;

/// A login user: name, secret (plaintext or argon2 PHC hash), roles.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub name: String,
    pub secret: String,
    pub roles: Vec<String>,
}

impl AuthUser {
    pub fn new(name: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            secret: secret.into(),
            roles: Vec::new(),
        }
    }

    pub fn roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    /// True when the stored secret is an argon2 PHC hash.
    pub fn is_hashed(&self) -> bool {
        self.secret.starts_with("$argon2")
    }

    /// Verify a candidate password against this user's secret.
    pub fn verify(&self, password: &str) -> bool {
        if self.is_hashed() {
            PasswordHash::new(&self.secret)
                .map(|hash| {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &hash)
                        .is_ok()
                })
                .unwrap_or(false)
        } else {
            self.secret == password
        }
    }
}

/// Parse the `FORGE_AUTH_USERS` format.
pub fn parse_users(raw: &str) -> Result<Vec<AuthUser>, ForgeError> {
    let mut users: Vec<AuthUser> = Vec::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Some((name, secret)) = entry.split_once(':') else {
            // Argon2 PHC hashes contain commas in their params
            // (`$argon2id$v=19$m=19456,t=2,p=1$…`), so a colon-less fragment
            // is the continuation of the previous entry's secret, not a new
            // user (user names always precede a colon).
            if let Some(last) = users.last_mut() {
                last.secret.push(',');
                last.secret.push_str(entry);
                continue;
            }
            return Err(ForgeError::Config(format!(
                "FORGE_AUTH_USERS entry {entry:?} has no colon (expected user:secret)"
            )));
        };
        let user = AuthUser::new(name, secret);
        if !user.is_hashed() {
            tracing::warn!(
                user = name,
                "FORGE_AUTH_USERS entry uses a plaintext password; prefer an argon2 hash (forge-hash)"
            );
        }
        users.push(user);
    }
    Ok(users)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_colon_splits() {
        let users = parse_users("admin:pa:ss,ops:$argon2id$xyz").unwrap();
        assert_eq!(users[0].name, "admin");
        assert_eq!(users[0].secret, "pa:ss");
        assert!(users[1].is_hashed());
    }

    #[test]
    fn missing_colon_errors() {
        assert!(parse_users("admin").is_err());
    }

    #[test]
    fn phc_hash_commas_stay_in_the_secret() {
        // forge-hash output: real PHC hashes carry commas in their params.
        let hash = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$YWJjZGVmZ2g";
        let users = parse_users(&format!("overseer:{hash},admin:pw")).unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].secret, hash);
        assert!(users[0].is_hashed());
        assert_eq!(users[1].name, "admin");
        assert_eq!(users[1].secret, "pw");
    }

    #[test]
    fn plaintext_verify() {
        let u = AuthUser::new("a", "secret");
        assert!(u.verify("secret"));
        assert!(!u.verify("nope"));
    }
}
