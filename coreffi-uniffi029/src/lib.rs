//! Synap Core UniFFI adapter for consumers pinned to UniFFI 0.29.4.

#[path = "../../coreffi-shared/src/adapter.rs"]
mod adapter;
#[path = "../../coreffi-shared/src/error.rs"]
mod error;
#[path = "../../coreffi-shared/src/service.rs"]
mod service;
#[path = "../../coreffi-shared/src/types.rs"]
mod types;

pub use adapter::*;

uniffi::include_scaffolding!("synap");
