use redb::ReadableTable;
use serde::{Deserialize, Serialize};

use crate::{
    db::{kvstore::KvStore, types::BlockId},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct UmapPointRecord {
    pub x: f32,
    pub y: f32,
}

const NOTE_UMAP_CACHE: KvStore<BlockId, UmapPointRecord> = KvStore::new("NoteUmapCache");

pub(crate) struct UmapCache;

impl UmapCache {
    pub fn init_schema(tx: &redb::WriteTransaction) -> Result<(), redb::Error> {
        NOTE_UMAP_CACHE.init_table(tx)
    }

    pub fn put(
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
        point: &UmapPointRecord,
    ) -> Result<(), redb::Error> {
        NOTE_UMAP_CACHE.put(tx, note_id, point)
    }

    pub fn delete(tx: &redb::WriteTransaction, note_id: &BlockId) -> Result<bool, redb::Error> {
        NOTE_UMAP_CACHE.delete(tx, note_id)
    }

    pub fn clear(tx: &redb::WriteTransaction) -> Result<usize, redb::Error> {
        let mut table = tx.open_table(NOTE_UMAP_CACHE.table_def())?;
        let keys = table
            .range::<BlockId>(..)?
            .map(|res| res.map(|(guard, _)| guard.value()))
            .collect::<Result<Vec<_>, _>>()?;

        let cleared = keys.len();
        for key in keys {
            table.remove(key)?;
        }

        Ok(cleared)
    }

    pub fn iter(
        tx: &redb::ReadTransaction,
    ) -> Result<
        impl Iterator<Item = Result<(BlockId, UmapPointRecord), redb::StorageError>> + '_,
        redb::Error,
    > {
        let reader = NOTE_UMAP_CACHE.reader(tx)?;
        let entries = reader
            .iter()?
            .map(|res| res.map(|(guard, point)| (guard.value(), point)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries.into_iter().map(Ok))
    }

    pub fn count(tx: &redb::ReadTransaction) -> Result<usize, redb::Error> {
        Ok(Self::iter(tx)?.count())
    }
}
