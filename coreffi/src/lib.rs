//! Synap Core FFI Compatibility Layer
//!
//! This module provides FFI bindings for the Synap core library using Uniffi.

// mod connection; // TODO: Fix connection module for uniffi - needs updates to work with new API
mod error;
mod service;
mod types;

pub use error::FfiError;
pub use service::{open, open_memory, SynapService};
pub use types::NoteDTO;

// Include uniffi bindings - this will generate the Kotlin bindings
uniffi::include_scaffolding!("synap");
