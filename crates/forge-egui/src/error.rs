//! Crate error type. Widgets are infallible; only the runtime (window
//! creation, event loop) can fail.

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("eframe: {0}")]
    Eframe(#[from] eframe::Error),
}
