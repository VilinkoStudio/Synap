use redb::ReadableTable;
use serde::{Deserialize, Serialize};

use crate::db::{kvstore::KvStore, types::BlockId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct UmapPointRecord {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct UmapAnchorRecord {
    pub note_id: BlockId,
    pub vector: Vec<f32>,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub(crate) struct UmapModelRecord {
    pub anchors: Vec<UmapAnchorRecord>,
    pub generation: u64,
}

const NOTE_UMAP_CACHE: KvStore<BlockId, UmapPointRecord> = KvStore::new("NoteUmapCache");
const UMAP_MODEL_CACHE: KvStore<u8, UmapModelRecord> = KvStore::new("UmapModelCache");
const MODEL_KEY: u8 = 0;

pub(crate) struct UmapCache;

impl UmapCache {
    pub fn init_schema(tx: &redb::WriteTransaction) -> Result<(), redb::Error> {
        NOTE_UMAP_CACHE.init_table(tx)?;
        UMAP_MODEL_CACHE.init_table(tx)?;
        Ok(())
    }

    pub fn put_point(
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
        point: &UmapPointRecord,
    ) -> Result<(), redb::Error> {
        NOTE_UMAP_CACHE.put(tx, note_id, point)
    }

    pub fn delete_point(
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
    ) -> Result<bool, redb::Error> {
        NOTE_UMAP_CACHE.delete(tx, note_id)
    }

    pub fn clear_points(tx: &redb::WriteTransaction) -> Result<usize, redb::Error> {
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

    pub fn iter_points(
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

    pub fn points_count(tx: &redb::ReadTransaction) -> Result<usize, redb::Error> {
        Ok(Self::iter_points(tx)?.count())
    }

    pub fn load_model(tx: &redb::ReadTransaction) -> Result<Option<UmapModelRecord>, redb::Error> {
        let reader = UMAP_MODEL_CACHE.reader(tx)?;
        reader.get(&MODEL_KEY)
    }

    pub fn save_model(
        tx: &redb::WriteTransaction,
        model: &UmapModelRecord,
    ) -> Result<(), redb::Error> {
        UMAP_MODEL_CACHE.put(tx, &MODEL_KEY, model)
    }

    pub fn clear_model(tx: &redb::WriteTransaction) -> Result<bool, redb::Error> {
        UMAP_MODEL_CACHE.delete(tx, &MODEL_KEY)
    }
}
