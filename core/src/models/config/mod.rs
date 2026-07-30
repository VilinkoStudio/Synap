//! Typed core configuration and its redb persistence boundary.

mod model;
mod store;

pub use model::{ConfigError, CoreConfig, EmbeddingConfig};
pub(crate) use store::ConfigWriter;
