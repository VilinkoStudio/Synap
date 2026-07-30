use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::db::kvstore::{KvReader, KvStore};

use super::model::DEFAULT_HTTP_TIMEOUT_MS;
use super::{ConfigError, CoreConfig, EmbeddingConfig};

const CONFIG_STORE: KvStore<u8, CoreConfigRecord> = KvStore::new("CoreConfig");
const CORE_CONFIG_KEY: u8 = 0;
const CONFIG_RECORD_VERSION: u32 = 1;
const DEFAULT_EMBEDDING_DIMENSION: usize = 384;

/// Private durable form. Runtime configuration remains independent from the
/// storage representation so future schema versions can migrate explicitly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CoreConfigRecord {
    version: u32,
    embedding: EmbeddingConfigRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum EmbeddingConfigRecord {
    LocalHash {
        dimension: usize,
    },
    Http {
        endpoint: String,
        api_key: String,
        model: String,
        dimension: usize,
        timeout_ms: u64,
    },
}

impl CoreConfigRecord {
    fn into_config(self) -> Result<CoreConfig, ConfigError> {
        match self.version {
            CONFIG_RECORD_VERSION => {
                let config = CoreConfig::new(EmbeddingConfig::from_record(self.embedding)?);
                config.validate()?;
                Ok(config)
            }
            version => Err(ConfigError::UnsupportedVersion(version)),
        }
    }

    fn from_config(config: &CoreConfig) -> Self {
        Self {
            version: CONFIG_RECORD_VERSION,
            embedding: EmbeddingConfigRecord::from_config(config.embedding()),
        }
    }
}

impl EmbeddingConfig {
    fn from_record(record: EmbeddingConfigRecord) -> Result<Self, ConfigError> {
        let config = match record {
            EmbeddingConfigRecord::LocalHash { dimension } => Self::LocalHash {
                dimension: if dimension == 0 {
                    DEFAULT_EMBEDDING_DIMENSION
                } else {
                    dimension
                },
            },
            EmbeddingConfigRecord::Http {
                endpoint,
                api_key,
                model,
                dimension,
                timeout_ms,
            } => Self::Http {
                endpoint: endpoint.trim().to_owned(),
                api_key: api_key.trim().to_owned(),
                model: model.trim().to_owned(),
                dimension: if dimension == 0 {
                    DEFAULT_EMBEDDING_DIMENSION
                } else {
                    dimension
                },
                timeout_ms: if timeout_ms == 0 {
                    DEFAULT_HTTP_TIMEOUT_MS
                } else {
                    timeout_ms
                },
            },
        };
        config.validate()?;
        Ok(config)
    }
}

impl EmbeddingConfigRecord {
    fn from_config(config: &EmbeddingConfig) -> Self {
        match config {
            EmbeddingConfig::LocalHash { dimension } => Self::LocalHash {
                dimension: *dimension,
            },
            EmbeddingConfig::Http {
                endpoint,
                api_key,
                model,
                dimension,
                timeout_ms,
            } => Self::Http {
                endpoint: endpoint.clone(),
                api_key: api_key.clone(),
                model: model.clone(),
                dimension: *dimension,
                timeout_ms: *timeout_ms,
            },
        }
    }
}

pub struct ConfigReader<'a> {
    table: KvReader<u8, CoreConfigRecord>,
    _marker: std::marker::PhantomData<&'a ReadTransaction>,
}

impl<'a> ConfigReader<'a> {
    pub fn new(tx: &'a ReadTransaction) -> Result<Self, redb::Error> {
        Ok(Self {
            table: CONFIG_STORE.reader(tx)?,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn load(&self) -> Result<Option<CoreConfig>, ConfigError> {
        self.table
            .get(&CORE_CONFIG_KEY)?
            .map(CoreConfigRecord::into_config)
            .transpose()
    }
}

pub(crate) struct ConfigWriter<'a> {
    tx: &'a WriteTransaction,
}

impl<'a> ConfigWriter<'a> {
    pub(crate) fn new(tx: &'a WriteTransaction) -> Self {
        Self { tx }
    }

    pub(crate) fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        CONFIG_STORE.init_table(tx)
    }

    pub(crate) fn save(&self, config: &CoreConfig) -> Result<(), ConfigError> {
        config.validate()?;
        CONFIG_STORE.put(
            self.tx,
            &CORE_CONFIG_KEY,
            &CoreConfigRecord::from_config(config),
        )?;
        Ok(())
    }

    pub(crate) fn load(&self) -> Result<Option<CoreConfig>, ConfigError> {
        CONFIG_STORE
            .get_in_write(self.tx, &CORE_CONFIG_KEY)?
            .map(CoreConfigRecord::into_config)
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    fn temp_db() -> Database {
        let file = NamedTempFile::new().unwrap();
        Database::create(file.path()).unwrap()
    }

    #[test]
    fn config_default_roundtrip() {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        ConfigWriter::init_schema(&wtx).unwrap();
        let config = CoreConfig::default();
        ConfigWriter::new(&wtx).save(&config).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        assert_eq!(
            ConfigReader::new(&rtx).unwrap().load().unwrap(),
            Some(config)
        );
    }

    #[test]
    fn config_http_roundtrip_and_validation() {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        ConfigWriter::init_schema(&wtx).unwrap();
        let config = CoreConfig::new(EmbeddingConfig::http(
            "https://api.openai.com/v1/embeddings",
            "sk-test",
            "text-embedding-3-small",
            1536,
            15_000,
        ));
        ConfigWriter::new(&wtx).save(&config).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        assert_eq!(
            ConfigReader::new(&rtx).unwrap().load().unwrap(),
            Some(config)
        );

        assert!(CoreConfig::new(EmbeddingConfig::http(" ", "sk", "m", 8, 1))
            .validate()
            .is_err());
    }

    #[test]
    fn legacy_zero_values_are_defaulted() {
        let config = CoreConfigRecord {
            version: CONFIG_RECORD_VERSION,
            embedding: EmbeddingConfigRecord::Http {
                endpoint: " https://example.test ".into(),
                api_key: " key ".into(),
                model: " model ".into(),
                dimension: 0,
                timeout_ms: 0,
            },
        }
        .into_config()
        .unwrap();

        assert_eq!(config.embedding().dimension(), DEFAULT_EMBEDDING_DIMENSION);
        assert_eq!(
            config.embedding(),
            &EmbeddingConfig::Http {
                endpoint: "https://example.test".into(),
                api_key: "key".into(),
                model: "model".into(),
                dimension: DEFAULT_EMBEDDING_DIMENSION,
                timeout_ms: DEFAULT_HTTP_TIMEOUT_MS,
            }
        );
    }

    #[test]
    fn unsupported_record_version_is_rejected() {
        let error = CoreConfigRecord {
            version: CONFIG_RECORD_VERSION + 1,
            embedding: EmbeddingConfigRecord::LocalHash { dimension: 8 },
        }
        .into_config()
        .unwrap_err();

        assert!(matches!(error, ConfigError::UnsupportedVersion(_)));
    }
}
