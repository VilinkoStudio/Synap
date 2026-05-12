use std::net::SocketAddr;

use anyhow::Context;
use clap::Parser;
use relay::{
    app::{AppState, AppStateParts, RelayAuth},
    cli::{Cli, Commands, EmbeddedRedisMode, ServeArgs},
    embedded_redis::EmbeddedRedisHandle,
    http::build_router,
    redis::RedisRuntime,
};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cli = Cli::parse();
    match cli.command {
        Commands::Serve(args) => serve(args).await,
    }
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let embedded_redis = start_embedded_redis_if_needed(&args).await?;
    let redis_runtime = RedisRuntime::new(args.redis_url.clone())?;
    let auth = relay_auth(&args)?;
    let app_state = AppState::from_parts(AppStateParts {
        server_name: args.server_name,
        redis_runtime,
        embedded_redis,
        auth,
    });

    let listener = bind_listener(args.listen).await?;
    let local_addr = listener
        .local_addr()
        .context("failed to resolve local listen address")?;
    let router = build_router(app_state);

    info!(
        listen = %local_addr,
        "relay HTTP server is ready"
    );

    let shutdown = shutdown_signal();

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
        .context("relay HTTP server exited unexpectedly")?;
    Ok(())
}

async fn bind_listener(listen: SocketAddr) -> anyhow::Result<TcpListener> {
    TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind relay HTTP listener on {listen}"))
}

async fn start_embedded_redis_if_needed(
    args: &ServeArgs,
) -> anyhow::Result<Option<EmbeddedRedisHandle>> {
    match args.embedded_redis {
        EmbeddedRedisMode::Disabled => Ok(None),
        EmbeddedRedisMode::Enabled => {
            let handle = EmbeddedRedisHandle::spawn(args.redis_listen)
                .await
                .with_context(|| {
                    format!(
                        "failed to start embedded mini-redis on {}",
                        args.redis_listen
                    )
                })?;

            info!(
                redis_addr = %handle.listen_addr(),
                "embedded mini-redis is enabled"
            );

            Ok(Some(handle))
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            warn!(error = %error, "failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        let mut signal =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(signal) => signal,
                Err(error) => {
                    warn!(error = %error, "failed to register SIGTERM handler");
                    return;
                }
            };
        signal.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    info!("shutdown signal received");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("relay=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn relay_auth(args: &ServeArgs) -> anyhow::Result<RelayAuth> {
    if args.no_key {
        return Ok(RelayAuth::Disabled);
    }

    let api_key = args
        .api_key
        .as_ref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("--api-key must not be empty unless --no-key is set"))?;
    Ok(RelayAuth::ApiKey(api_key.clone()))
}
