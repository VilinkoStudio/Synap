use std::fmt;

use sha2::{Digest, Sha256};

const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
pub(crate) const DEFAULT_HTTP_TIMEOUT_MS: u64 = 30_000;

/// Runtime configuration owned by [`crate::service::SynapService`].
///
/// Fields are intentionally private so adding future configuration domains does
/// not require callers to construct every field directly.
#[derive(Clone, PartialEq, Eq)]
pub struct CoreConfig {
    embedding: EmbeddingConfig,
}

impl CoreConfig {
    pub fn new(embedding: EmbeddingConfig) -> Self {
        Self { embedding }
    }

    pub fn default_local() -> Self {
        Self::new(EmbeddingConfig::LocalHash {
            dimension: DEFAULT_EMBEDDING_DIMENSION,
        })
    }

    pub fn embedding(&self) -> &EmbeddingConfig {
        &self.embedding
    }

    pub fn set_embedding(&mut self, embedding: EmbeddingConfig) {
        self.embedding = embedding;
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.embedding.validate()
    }
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self::default_local()
    }
}

impl fmt::Debug for CoreConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoreConfig")
            .field("embedding", &self.embedding)
            .finish()
    }
}

/// Embedding provider configuration.
#[derive(Clone, PartialEq, Eq)]
pub enum EmbeddingConfig {
    LocalHash {
        dimension: usize,
    },
    /// OpenAI-compatible HTTP embeddings API.
    Http {
        endpoint: String,
        api_key: String,
        model: String,
        dimension: usize,
        timeout_ms: u64,
    },
}

impl EmbeddingConfig {
    pub fn local_hash(dimension: usize) -> Self {
        Self::LocalHash { dimension }
    }

    pub fn http(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        dimension: usize,
        timeout_ms: u64,
    ) -> Self {
        Self::Http {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            dimension,
            timeout_ms,
        }
    }

    pub fn dimension(&self) -> usize {
        match self {
            Self::LocalHash { dimension } | Self::Http { dimension, .. } => *dimension,
        }
    }

    /// Stable identity for vectors derived from this configuration. Secrets and
    /// transport-only settings intentionally do not participate.
    pub(crate) fn space_fingerprint(&self) -> String {
        match self {
            Self::LocalHash { dimension } => format!("local-hash:v1:{dimension}"),
            Self::Http {
                endpoint,
                model,
                dimension,
                ..
            } => {
                let digest = Sha256::digest(
                    format!("{}\0{}\0{dimension}", endpoint.trim(), model.trim()).as_bytes(),
                );
                format!("http:v1:{digest:x}")
            }
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            Self::LocalHash { dimension } => {
                if *dimension == 0 {
                    return Err(ConfigError::Invalid(
                        "local hash embedding dimension must be > 0".into(),
                    ));
                }
            }
            Self::Http {
                endpoint,
                api_key,
                model,
                dimension,
                timeout_ms,
            } => {
                if endpoint.trim().is_empty() {
                    return Err(ConfigError::Invalid(
                        "http embedding endpoint cannot be empty".into(),
                    ));
                }
                if api_key.trim().is_empty() {
                    return Err(ConfigError::Invalid(
                        "http embedding api key cannot be empty".into(),
                    ));
                }
                if model.trim().is_empty() {
                    return Err(ConfigError::Invalid(
                        "http embedding model cannot be empty".into(),
                    ));
                }
                if *dimension == 0 {
                    return Err(ConfigError::Invalid(
                        "http embedding dimension must be > 0".into(),
                    ));
                }
                if *timeout_ms == 0 {
                    return Err(ConfigError::Invalid(
                        "http embedding timeout_ms must be > 0".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl fmt::Debug for EmbeddingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalHash { dimension } => f
                .debug_struct("LocalHash")
                .field("dimension", dimension)
                .finish(),
            Self::Http {
                endpoint,
                model,
                dimension,
                timeout_ms,
                ..
            } => f
                .debug_struct("Http")
                .field("endpoint", endpoint)
                .field("api_key", &"<redacted>")
                .field("model", model)
                .field("dimension", dimension)
                .field("timeout_ms", timeout_ms)
                .finish(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("database error: {0}")]
    Db(#[from] redb::Error),

    #[error("unsupported config version: {0}")]
    UnsupportedVersion(u32),

    #[error("invalid config: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_debug_redacts_api_key() {
        let config = EmbeddingConfig::http("https://example.test", "secret", "model", 8, 1);
        let debug = format!("{config:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret"));
    }
}
