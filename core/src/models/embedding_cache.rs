use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::db::kvstore::{KvReader, KvStore};

const EMBEDDING_CACHE_METADATA: KvStore<u8, EmbeddingCacheStamp> =
    KvStore::new("EmbeddingCacheMetadata");
const METADATA_KEY: u8 = 0;

pub(crate) const EMBEDDING_CACHE_SCHEMA_VERSION: u32 = 1;
pub(crate) const EMBEDDING_VECTOR_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EmbeddingCacheStamp {
    pub schema_version: u32,
    pub vector_format_version: u32,
    pub space_fingerprint: String,
    pub dimension: u32,
}

impl EmbeddingCacheStamp {
    pub fn new(space_fingerprint: String, dimension: usize) -> Self {
        Self {
            schema_version: EMBEDDING_CACHE_SCHEMA_VERSION,
            vector_format_version: EMBEDDING_VECTOR_FORMAT_VERSION,
            space_fingerprint,
            dimension: dimension as u32,
        }
    }

    pub fn is_compatible_with(&self, expected: &Self) -> bool {
        self == expected
    }
}

pub(crate) struct EmbeddingCacheMetadata;

impl EmbeddingCacheMetadata {
    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        EMBEDDING_CACHE_METADATA.init_table(tx)
    }

    pub fn load(tx: &ReadTransaction) -> Result<Option<EmbeddingCacheStamp>, redb::Error> {
        let reader: KvReader<u8, EmbeddingCacheStamp> = EMBEDDING_CACHE_METADATA.reader(tx)?;
        reader.get(&METADATA_KEY)
    }

    pub fn save(tx: &WriteTransaction, stamp: &EmbeddingCacheStamp) -> Result<(), redb::Error> {
        EMBEDDING_CACHE_METADATA.put(tx, &METADATA_KEY, stamp)
    }
}

#[cfg(test)]
mod tests {
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn embedding_cache_stamp_roundtrips() {
        let file = NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        let tx = db.begin_write().unwrap();
        EmbeddingCacheMetadata::init_schema(&tx).unwrap();
        let stamp = EmbeddingCacheStamp::new("local-hash:v1:384".into(), 384);
        EmbeddingCacheMetadata::save(&tx, &stamp).unwrap();
        tx.commit().unwrap();

        let tx = db.begin_read().unwrap();
        assert_eq!(EmbeddingCacheMetadata::load(&tx).unwrap(), Some(stamp));
    }
}
