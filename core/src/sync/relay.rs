use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ServiceError, models::note::NoteRecord, service::SynapService};

use super::{
    protocol::{SyncError, SyncRecordId},
    share::SharePackage,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayInventory {
    pub version: u8,
    pub records: Vec<RelayRecordDescriptor>,
}

impl RelayInventory {
    pub const VERSION: u8 = 1;

    pub fn validate(&self) -> Result<(), SyncError> {
        if self.version != Self::VERSION {
            return Err(SyncError::InvalidManifest(format!(
                "unsupported relay inventory version: {}",
                self.version
            )));
        }

        let mut seen = BTreeSet::new();
        for descriptor in &self.records {
            if !seen.insert(descriptor.sync_id) {
                return Err(SyncError::InvalidManifest(format!(
                    "duplicate relay sync record id: {:?}",
                    descriptor.sync_id
                )));
            }
        }

        Ok(())
    }

    pub fn sync_ids(&self) -> BTreeSet<SyncRecordId> {
        self.records
            .iter()
            .map(|descriptor| descriptor.sync_id)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelayRecordDescriptor {
    pub root_note_id: Uuid,
    pub sync_id: SyncRecordId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayDiffPlan {
    pub remote_missing_local_records: Vec<RelayRecordDescriptor>,
    pub local_missing_remote_records: Vec<RelayRecordDescriptor>,
}

pub struct RelaySyncService<'a> {
    core: &'a SynapService,
}

impl<'a> RelaySyncService<'a> {
    pub fn new(core: &'a SynapService) -> Self {
        Self { core }
    }

    pub fn build_local_inventory(&self) -> Result<RelayInventory, SyncError> {
        let descriptors = self.collect_local_descriptors()?;
        Ok(RelayInventory {
            version: RelayInventory::VERSION,
            records: descriptors,
        })
    }

    pub fn plan_diff_against_remote(
        &self,
        remote: &RelayInventory,
    ) -> Result<RelayDiffPlan, SyncError> {
        remote.validate()?;

        let local = self.collect_local_descriptors()?;
        let local_by_sync_id: BTreeMap<SyncRecordId, RelayRecordDescriptor> = local
            .iter()
            .copied()
            .map(|descriptor| (descriptor.sync_id, descriptor))
            .collect();
        let remote_by_sync_id: BTreeMap<SyncRecordId, RelayRecordDescriptor> = remote
            .records
            .iter()
            .copied()
            .map(|descriptor| (descriptor.sync_id, descriptor))
            .collect();

        let remote_missing_local_records = local_by_sync_id
            .iter()
            .filter_map(|(sync_id, descriptor)| {
                (!remote_by_sync_id.contains_key(sync_id)).then_some(*descriptor)
            })
            .collect();

        let local_missing_remote_records = remote_by_sync_id
            .iter()
            .filter_map(|(sync_id, descriptor)| {
                (!local_by_sync_id.contains_key(sync_id)).then_some(*descriptor)
            })
            .collect();

        Ok(RelayDiffPlan {
            remote_missing_local_records,
            local_missing_remote_records,
        })
    }

    pub fn build_share_for_remote_inventory(
        &self,
        remote: &RelayInventory,
    ) -> Result<Vec<u8>, ServiceError> {
        let diff = self
            .plan_diff_against_remote(remote)
            .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))?;
        let note_ids: Vec<Uuid> = diff
            .remote_missing_local_records
            .iter()
            .map(|descriptor| descriptor.root_note_id)
            .collect();
        let records = self.export_records(&note_ids)?;
        SharePackage::new(records)
            .encode()
            .map_err(|err| ServiceError::ShareProtocol(err.to_string()))
    }

    fn collect_local_descriptors(&self) -> Result<Vec<RelayRecordDescriptor>, SyncError> {
        let records = self.core.with_read(|_tx, reader| {
            let note_ids = reader
                .note_by_time()
                .map_err(redb::Error::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?;

            reader.export_records(&note_ids).map_err(Into::into)
        })?;

        build_descriptors_from_records(records)
    }

    fn export_records(&self, note_ids: &[Uuid]) -> Result<Vec<NoteRecord>, ServiceError> {
        self.core
            .with_read(|_tx, reader| reader.export_records(note_ids).map_err(Into::into))
    }
}

fn build_descriptors_from_records(
    records: Vec<NoteRecord>,
) -> Result<Vec<RelayRecordDescriptor>, SyncError> {
    let mut descriptors = Vec::with_capacity(records.len());
    let mut seen = BTreeSet::new();

    for record in records {
        let sync_id = SyncRecordId::for_record(&record)?;
        if !seen.insert(sync_id) {
            return Err(SyncError::RecordIdCollision { record_id: sync_id });
        }
        descriptors.push(RelayRecordDescriptor {
            root_note_id: record.id,
            sync_id,
        });
    }

    descriptors.sort_by_key(|descriptor| descriptor.sync_id);
    Ok(descriptors)
}

#[cfg(test)]
mod tests {
    use crate::{service::SynapService, sync::ShareService};

    use super::*;
    use crate::sync::share::SharePackage;

    fn make_note(service: &SynapService, content: &str) -> Uuid {
        service
            .create_note(content.to_owned(), Vec::new())
            .unwrap()
            .id
            .parse()
            .unwrap()
    }

    #[test]
    fn build_local_inventory_lists_sync_ids() {
        let service = SynapService::open_memory().unwrap();
        let note_id = make_note(&service, "relay a");
        let relay = RelaySyncService::new(&service);

        let inventory = relay.build_local_inventory().unwrap();

        assert_eq!(inventory.version, RelayInventory::VERSION);
        assert_eq!(inventory.records.len(), 1);
        assert_eq!(inventory.records[0].root_note_id, note_id);
    }

    #[test]
    fn plan_diff_detects_missing_records_on_both_sides() {
        let local = SynapService::open_memory().unwrap();
        let remote = SynapService::open_memory().unwrap();

        let local_only = make_note(&local, "local only");
        let remote_only = make_note(&remote, "remote only");

        let remote_inventory = RelaySyncService::new(&remote)
            .build_local_inventory()
            .unwrap();
        let diff = RelaySyncService::new(&local)
            .plan_diff_against_remote(&remote_inventory)
            .unwrap();

        assert_eq!(diff.remote_missing_local_records.len(), 1);
        assert_eq!(
            diff.remote_missing_local_records[0].root_note_id,
            local_only
        );
        assert_eq!(diff.local_missing_remote_records.len(), 1);
        assert_eq!(
            diff.local_missing_remote_records[0].root_note_id,
            remote_only
        );
    }

    #[test]
    fn build_share_for_remote_inventory_exports_only_missing_local_records() {
        let local = SynapService::open_memory().unwrap();
        let remote = SynapService::open_memory().unwrap();

        let shared_id = make_note(&local, "shared");
        let local_only_id = make_note(&local, "local only");

        let shared_share = ShareService::new(&local)
            .export_bytes(&[shared_id])
            .unwrap();
        ShareService::new(&remote)
            .import_bytes(&shared_share)
            .unwrap();

        let remote_inventory = RelaySyncService::new(&remote)
            .build_local_inventory()
            .unwrap();
        let share = RelaySyncService::new(&local)
            .build_share_for_remote_inventory(&remote_inventory)
            .unwrap();
        let package = SharePackage::decode(&share).unwrap();

        assert_eq!(package.records.len(), 1);
        assert_eq!(package.records[0].id, local_only_id);
    }
}
