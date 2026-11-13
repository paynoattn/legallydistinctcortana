//! Binary entry point: reads `HOST`/`PORT`, serves until SIGINT or SIGTERM.
//!
//! Kept to wiring only. It is excluded from unit coverage and mutation testing and is
//! exercised end to end by `make qa`; logic belongs in `lib.rs`, where it is tested.

use std::env;
use std::io;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    let host = env::var("HOST").ok();
    let port = env::var("PORT").ok();
    let addr = app::listen_addr(host.as_deref(), port.as_deref())?;
    let shutdown = shutdown_signal()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");
    axum::serve(listener, app::router())
        .with_graceful_shutdown(shutdown)
        .await?;
    tracing::info!("shut down");
    Ok(())
}

#[cfg(unix)]
fn shutdown_signal() -> io::Result<impl Future<Output = ()>> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    })
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "same signature as the unix variant so main stays platform-agnostic"
)]
fn shutdown_signal() -> io::Result<impl Future<Output = ()>> {
    Ok(async {
        drop(tokio::signal::ctrl_c().await);
    })
}
