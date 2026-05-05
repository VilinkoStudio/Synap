use std::net::SocketAddr;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "synap-relay",
    about = "Zero-trust relay service for cross-network sync",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the relay HTTP server.
    Serve(ServeArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct ServeArgs {
    /// Relay HTTP listen address.
    #[arg(long, env = "SYNAP_RELAY_LISTEN", default_value = "127.0.0.1:8787")]
    pub listen: SocketAddr,

    /// Human-friendly server name returned by diagnostics endpoints.
    #[arg(long, env = "SYNAP_RELAY_SERVER_NAME", default_value = "synap-relay")]
    pub server_name: String,

    /// Redis connection URL.
    #[arg(
        long,
        env = "SYNAP_RELAY_REDIS_URL",
        default_value = "redis://127.0.0.1:6379/"
    )]
    pub redis_url: String,

    /// Whether to start an embedded mini-redis instance.
    #[arg(
        long,
        env = "SYNAP_RELAY_EMBEDDED_REDIS",
        value_enum,
        default_value_t = EmbeddedRedisMode::Disabled
    )]
    pub embedded_redis: EmbeddedRedisMode,

    /// Listen address for the embedded mini-redis instance.
    #[arg(
        long,
        env = "SYNAP_RELAY_REDIS_LISTEN",
        default_value = "127.0.0.1:6379"
    )]
    pub redis_listen: SocketAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EmbeddedRedisMode {
    Disabled,
    Enabled,
}
