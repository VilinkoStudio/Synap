//! Synap Core FFI Compatibility Layer
//!
//! This module provides FFI bindings for the Synap core library using Uniffi.

// mod connection; // TODO: Fix connection module for uniffi - needs updates to work with new API
mod error;
mod service;
mod types;

pub trait SyncTransport: Send + Sync {
    fn read(&self, max_bytes: u32) -> Result<Vec<u8>, FfiError>;
    fn write(&self, payload: Vec<u8>) -> Result<(), FfiError>;
    fn close(&self) -> Result<(), FfiError>;
}

pub use error::FfiError;
pub use service::{get_build_info, get_version_string, open, open_memory, SynapService};
pub use types::{
    BuildInfo, FilteredNoteStatus, LocalIdentityDTO, NoteBriefDTO, NoteContentDiffStatsDTO,
    NoteDTO, NoteNeighborContextDTO, NoteNeighborsDTO, NoteSegmentBranchChoiceDTO, NoteSegmentDTO,
    NoteSegmentDirectionDTO, NoteSegmentStepDTO, NoteTagDiffDTO, NoteTextChangeDTO,
    NoteTextChangeKindDTO, NoteVersionDTO, NoteVersionDiffDTO, PeerDTO, PeerTrustStatusDTO,
    PublicKeyInfoDTO, RelayFetchStatsDTO, RelayPushStatsDTO, SearchResultDTO, SearchSourceDTO,
    ShareStatsDTO, StarmapPointDTO, SyncSessionDTO, SyncSessionRecordDTO, SyncSessionRoleDTO,
    SyncStatsDTO, SyncStatusDTO, SyncTransportKindDTO, TimelineDirection, TimelineNotesPageDTO,
    TimelineSessionDTO, TimelineSessionsPageDTO,
};

// Include uniffi bindings - this will generate the Kotlin bindings
uniffi::include_scaffolding!("synap");
