use forge_server::ForgeApp;
use forge_auth::{app, config::Config, error::AppError, init_state, spawn_sweeper};

/// Release builds embed the built SPA; debug builds serve `apps/auth/dist`
/// from disk (relative to the repo root — run via `just auth-dev`) so
/// `vite build --watch` (or `just frontend-build`) is picked up without
/// recompiling.
#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../apps/auth/dist"]
struct Assets;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let host = cfg.host.clone();
    let port = cfg.port;
    let issuer = cfg.issuer.clone();
    let state = init_state(cfg).await?;
    spawn_sweeper(state.clone());

    let forge_app = ForgeApp::new("forge-auth");
    #[cfg(debug_assertions)]
    let forge_app = forge_app.frontend_dir("apps/auth/dist");
    #[cfg(not(debug_assertions))]
    let forge_app = forge_app.frontend_embedded::<Assets>();

    let router = app::build_router(state, forge_app)?;

    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .map_err(|e| AppError::Config(format!("failed to bind {host}:{port}: {e}")))?;
    tracing::info!(%host, port, issuer, "forge-auth listening");
    axum::serve(listener, router)
        .await
        .map_err(AppError::internal)?;
    Ok(())
}
