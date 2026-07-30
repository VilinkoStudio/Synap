use redb::{Database, MultimapTableHandle, ReadableDatabase, ReadableTableMetadata, TableHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseStorageMetrics {
    pub tree_height: u32,
    pub allocated_pages: u64,
    pub leaf_pages: u64,
    pub branch_pages: u64,
    pub stored_bytes: u64,
    pub metadata_bytes: u64,
    pub fragmented_bytes: u64,
    pub page_size: u64,
    pub tables: Vec<TableStorageMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStorageMetrics {
    pub name: String,
    pub kind: TableStorageKind,
    pub entries: u64,
    pub tree_height: u32,
    pub leaf_pages: u64,
    pub branch_pages: u64,
    pub stored_bytes: u64,
    pub metadata_bytes: u64,
    pub fragmented_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStorageKind {
    Table,
    MultimapTable,
}

/// Collects a redb storage snapshot without interpreting application data.
pub fn collect_database_storage_metrics(
    db: &Database,
) -> Result<DatabaseStorageMetrics, redb::Error> {
    // redb exposes database-wide page statistics on a write transaction. The
    // transaction is only used for this read-only inspection and is dropped
    // before opening the table snapshot. redb does not expose database-wide
    // stats on a read transaction.
    let database_stats = {
        let write_tx = db.begin_write()?;
        write_tx.stats()?
    };
    let read_tx = db.begin_read()?;
    let mut tables = Vec::new();

    for handle in read_tx.list_tables()? {
        let name = handle.name().to_owned();
        let table = read_tx.open_untyped_table(handle)?;
        let stats = table.stats()?;
        tables.push(TableStorageMetrics {
            name,
            kind: TableStorageKind::Table,
            entries: table.len()?,
            tree_height: stats.tree_height(),
            leaf_pages: stats.leaf_pages(),
            branch_pages: stats.branch_pages(),
            stored_bytes: stats.stored_bytes(),
            metadata_bytes: stats.metadata_bytes(),
            fragmented_bytes: stats.fragmented_bytes(),
        });
    }

    for handle in read_tx.list_multimap_tables()? {
        let name = handle.name().to_owned();
        let table = read_tx.open_untyped_multimap_table(handle)?;
        let stats = table.stats()?;
        tables.push(TableStorageMetrics {
            name,
            kind: TableStorageKind::MultimapTable,
            entries: table.len()?,
            tree_height: stats.tree_height(),
            leaf_pages: stats.leaf_pages(),
            branch_pages: stats.branch_pages(),
            stored_bytes: stats.stored_bytes(),
            metadata_bytes: stats.metadata_bytes(),
            fragmented_bytes: stats.fragmented_bytes(),
        });
    }

    tables.sort_unstable_by(|left, right| left.name.cmp(&right.name));

    Ok(DatabaseStorageMetrics {
        tree_height: database_stats.tree_height(),
        allocated_pages: database_stats.allocated_pages(),
        leaf_pages: database_stats.leaf_pages(),
        branch_pages: database_stats.branch_pages(),
        stored_bytes: database_stats.stored_bytes(),
        metadata_bytes: database_stats.metadata_bytes(),
        fragmented_bytes: database_stats.fragmented_bytes(),
        page_size: database_stats.page_size() as u64,
        tables,
    })
}

#[cfg(test)]
mod tests {
    use redb::{Database, MultimapTableDefinition, TableDefinition};
    use tempfile::NamedTempFile;

    use super::{collect_database_storage_metrics, TableStorageKind};

    const TABLE: TableDefinition<u64, u64> = TableDefinition::new("metrics_table");
    const MULTIMAP: MultimapTableDefinition<u64, u64> =
        MultimapTableDefinition::new("metrics_multimap");

    #[test]
    fn collects_all_materialized_table_kinds() {
        let file = NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        let write_tx = db.begin_write().unwrap();
        {
            let mut table = write_tx.open_table(TABLE).unwrap();
            table.insert(1, 10).unwrap();
            table.insert(2, 20).unwrap();
        }
        {
            let mut table = write_tx.open_multimap_table(MULTIMAP).unwrap();
            table.insert(1, 10).unwrap();
            table.insert(1, 20).unwrap();
        }
        write_tx.commit().unwrap();

        let metrics = collect_database_storage_metrics(&db).unwrap();

        assert!(metrics.page_size > 0);
        assert!(metrics.allocated_pages > 0);
        assert_eq!(metrics.tables.len(), 2);
        assert_eq!(metrics.tables[0].name, "metrics_multimap");
        assert_eq!(metrics.tables[0].kind, TableStorageKind::MultimapTable);
        assert_eq!(metrics.tables[0].entries, 2);
        assert_eq!(metrics.tables[1].name, "metrics_table");
        assert_eq!(metrics.tables[1].kind, TableStorageKind::Table);
        assert_eq!(metrics.tables[1].entries, 2);
    }
}
