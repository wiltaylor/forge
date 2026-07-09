//! Admin REST API (`/api/admin/*`), guarded by the [`crate::session::AdminUser`]
//! extractor (session or Bearer token with the `admin` role).

pub mod clients;
pub mod keys;
pub mod providers;
pub mod roles;
pub mod sessions;
pub mod users;
