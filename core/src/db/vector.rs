use redb::{ReadTransaction, ReadableTable, TableDefinition, WriteTransaction};
use serde::{de::DeserializeOwned, Serialize};

use crate::db::types::BlockId;

const DEFAULT_VECTOR_DIM: usize = 384;

pub struct VectorStore<V: Serialize + DeserializeOwned> {
    def: TableDefinition<'static, BlockId, &'static [u8]>,
    dim: usize,
    _marker: std::marker::PhantomData<V>,
}

impl<V: Serialize + DeserializeOwned> VectorStore<V> {
    pub const fn new(name: &'static str, dimension: usize) -> Self {
        Self {
            def: TableDefinition::new(name),
            dim: if dimension > 0 {
                dimension
            } else {
                DEFAULT_VECTOR_DIM
            },
            _marker: std::marker::PhantomData,
        }
    }

    pub fn dimension(&self) -> usize {
        self.dim
    }

    pub fn table_def(&self) -> TableDefinition<'static, BlockId, &'static [u8]> {
        self.def
    }

    pub fn put(&self, tx: &WriteTransaction, key: &[u8; 16], value: &V) -> Result<(), redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let bytes = postcard::to_allocvec(value).expect("serialize failed");
        table.insert(key, bytes.as_slice())?;
        Ok(())
    }

    pub fn delete(&self, tx: &WriteTransaction, key: &[u8; 16]) -> Result<bool, redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let result = table.remove(key)?.is_some();
        Ok(result)
    }

    pub fn get(&self, tx: &ReadTransaction, key: &[u8; 16]) -> Result<Option<V>, redb::Error> {
        let table = tx.open_table(self.def)?;
        match table.get(key)? {
            Some(guard) => {
                let v = postcard::from_bytes(guard.value()).expect("deserialize failed");
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    pub fn iter(&self, tx: &ReadTransaction) -> Result<VectorIter<'_, V>, redb::Error> {
        let table = tx.open_table(self.def)?;
        let range = table.range::<BlockId>(..)?;
        Ok(VectorIter {
            inner: range,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn clear(&self, tx: &WriteTransaction) -> Result<usize, redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let keys = table
            .range::<BlockId>(..)?
            .map(|res| res.map(|(key_guard, _)| key_guard.value()))
            .collect::<Result<Vec<_>, _>>()?;

        let cleared = keys.len();
        for key in keys {
            table.remove(key)?;
        }

        Ok(cleared)
    }

    pub fn init_table(&self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        let _ = tx.open_table(self.def)?;
        Ok(())
    }
}

pub struct VectorIter<'a, V> {
    inner: redb::Range<'a, BlockId, &'static [u8]>,
    _marker: std::marker::PhantomData<V>,
}

impl<'a, V: DeserializeOwned> Iterator for VectorIter<'a, V> {
    type Item = Result<(redb::AccessGuard<'a, BlockId>, V), redb::StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map(|(k_guard, v_guard)| {
                let value = postcard::from_bytes(v_guard.value()).expect("deserialize failed");
                (k_guard, value)
            })
        })
    }
}
