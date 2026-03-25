use tokio_util::sync::CancellationToken;
use tracing::info;

/// Returns a [`CancellationToken`] that is cancelled when the process receives
/// SIGINT (Ctrl+C) or SIGTERM.
pub fn cancellation_token() -> CancellationToken {
    let token = CancellationToken::new();
    let t = token.clone();

    tokio::spawn(async move {
        wait_for_signal().await;
        info!("Shutdown signal received, initiating graceful shutdown");
        t.cancel();
    });

    token
}

async fn wait_for_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler");

        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}
