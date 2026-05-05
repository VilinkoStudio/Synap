//! `corenet` provides a Rust-native transport/runtime layer that stays outside
//! of `synap-core`.
//!
//! The first version intentionally stays small:
//! - TCP connect/listen
//! - listener lifecycle management
//! - adapting transport channels into `synap-core` sync entrypoints

mod channel;
mod discovery;
mod error;
mod runtime;
mod sync_service;

pub use channel::TcpChannel;
pub use discovery::{DiscoveredPeer, DiscoveryConfig, DiscoveryState, SyncDiscoveryRuntime};
pub use error::{DiscoveryError, NetError, SyncNetError};
pub use runtime::{
    spawn_incoming_loop, ConnectConfig, IncomingConnection, IncomingLoopHandle, ListenConfig,
    ListenerState, TcpListenerRuntime, TcpNetRuntime,
};
pub use sync_service::{SyncAcceptLoopHandle, SyncNetService};
