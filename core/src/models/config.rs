use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::db::kvstore::{KvReader, KvStore};

const CONFIG_STORE: KvStore<u8, CoreConfigRecord> = KvStore::new("CoreConfig");
const CORE_CONFIG_KEY: u8 = 0;
const CONFIG_RECORD_VERSION: u32 = 1;
const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
const DEFAULT_HTTP_EMBEDDING_ENDPOINT: &str = "https://api.openai.com/v1/embeddings";
const DEFAULT_HTTP_EMBEDDING_MODEL: &str = "text-embedding-3-small";
const DEFAULT_HTTP_TIMEOUT_MS: u64 = 30_000;

/// 持久化形态：带版本号，加载时校验并转成强类型 [`CoreConfig`]。
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

/// 运行时强类型配置（service 持有）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreConfig {
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingConfig {
    LocalHash {
        dimension: usize,
    },
    /// OpenAI-compatible HTTP embeddings API
    Http {
        endpoint: String,
        api_key: String,
        model: String,
        dimension: usize,
        timeout_ms: u64,
    },
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

impl CoreConfig {
    pub fn default_local() -> Self {
        Self {
            embedding: EmbeddingConfig::LocalHash {
                dimension: DEFAULT_EMBEDDING_DIMENSION,
            },
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.embedding.validate()
    }

    fn from_record(record: CoreConfigRecord) -> Result<Self, ConfigError> {
        if record.version != CONFIG_RECORD_VERSION {
            return Err(ConfigError::UnsupportedVersion(record.version));
        }

        let config = Self {
            embedding: EmbeddingConfig::from_record(record.embedding)?,
        };
        config.validate()?;
        Ok(config)
    }

    fn to_record(&self) -> CoreConfigRecord {
        CoreConfigRecord {
            version: CONFIG_RECORD_VERSION,
            embedding: self.embedding.to_record(),
        }
    }
}

impl EmbeddingConfig {
    pub fn dimension(&self) -> usize {
        match self {
            Self::LocalHash { dimension } | Self::Http { dimension, .. } => *dimension,
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

    fn to_record(&self) -> EmbeddingConfigRecord {
        match self {
            Self::LocalHash { dimension } => EmbeddingConfigRecord::LocalHash {
                dimension: *dimension,
            },
            Self::Http {
                endpoint,
                api_key,
                model,
                dimension,
                timeout_ms,
            } => EmbeddingConfigRecord::Http {
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

    /// 读取并校验为强类型。不存在时返回 `None`。
    pub fn load(&self) -> Result<Option<CoreConfig>, ConfigError> {
        let Some(record) = self.table.get(&CORE_CONFIG_KEY)? else {
            return Ok(None);
        };
        CoreConfig::from_record(record).map(Some)
    }
}

pub struct ConfigWriter<'a> {
    tx: &'a WriteTransaction,
}

impl<'a> ConfigWriter<'a> {
    pub fn new(tx: &'a WriteTransaction) -> Self {
        Self { tx }
    }

    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        CONFIG_STORE.init_table(tx)
    }

    pub fn save(&self, config: &CoreConfig) -> Result<(), ConfigError> {
        config.validate()?;
        CONFIG_STORE.put(self.tx, &CORE_CONFIG_KEY, &config.to_record())?;
        Ok(())
    }

    pub fn load(&self) -> Result<Option<CoreConfig>, ConfigError> {
        let Some(record) = CONFIG_STORE.get_in_write(self.tx, &CORE_CONFIG_KEY)? else {
            return Ok(None);
        };
        CoreConfig::from_record(record).map(Some)
    }

    /// 加载已有配置；缺失则写入默认本地 hash embedding 配置。
    pub fn load_or_default(&self) -> Result<CoreConfig, ConfigError> {
        if let Some(config) = self.load()? {
            return Ok(config);
        }

        let config = CoreConfig::default_local();
        self.save(&config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    #[test]
    fn test_config_default_roundtrip() {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        ConfigWriter::init_schema(&wtx).unwrap();
        let writer = ConfigWriter::new(&wtx);
        let config = writer.load_or_default().unwrap();
        assert_eq!(
            config.embedding,
            EmbeddingConfig::LocalHash {
                dimension: DEFAULT_EMBEDDING_DIMENSION
            }
        );
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let loaded = ConfigReader::new(&rtx).unwrap().load().unwrap().unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn test_config_http_roundtrip_and_validation() {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        ConfigWriter::init_schema(&wtx).unwrap();
        let writer = ConfigWriter::new(&wtx);

        let config = CoreConfig {
            embedding: EmbeddingConfig::Http {
                endpoint: DEFAULT_HTTP_EMBEDDING_ENDPOINT.into(),
                api_key: "sk-test".into(),
                model: DEFAULT_HTTP_EMBEDDING_MODEL.into(),
                dimension: 1536,
                timeout_ms: 15_000,
            },
        };
        writer.save(&config).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let loaded = ConfigReader::new(&rtx).unwrap().load().unwrap().unwrap();
        assert_eq!(loaded, config);

        let bad = CoreConfig {
            embedding: EmbeddingConfig::Http {
                endpoint: " ".into(),
                api_key: "sk".into(),
                model: "m".into(),
                dimension: 8,
                timeout_ms: 1,
            },
        };
        assert!(bad.validate().is_err());
    }
}
