//! Shared UniFFI adapter surface re-exported by version-specific crates.

pub trait SyncTransport: Send + Sync {
    fn read(&self, max_bytes: u32) -> Result<Vec<u8>, FfiError>;
    fn write(&self, payload: Vec<u8>) -> Result<(), FfiError>;
    fn close(&self) -> Result<(), FfiError>;
}

pub use crate::error::FfiError;
pub use crate::service::{get_build_info, get_version_string, open, open_memory, SynapService};
pub use crate::types::{
    BuildInfo, FilteredNoteStatus, LocalIdentityDTO, NoteBriefDTO, NoteContentDiffStatsDTO,
    NoteDTO, NoteNeighborContextDTO, NoteNeighborsDTO, NoteSegmentBranchChoiceDTO, NoteSegmentDTO,
    NoteSegmentDirectionDTO, NoteSegmentStepDTO, NoteTagDiffDTO, NoteTextChangeDTO,
    NoteTextChangeKindDTO, NoteVersionDTO, NoteVersionDiffDTO, PeerDTO, PeerTrustStatusDTO,
    PublicKeyInfoDTO, RelayFetchStatsDTO, RelayPushStatsDTO, SearchResultDTO, SearchSourceDTO,
    ShareStatsDTO, StarmapPointDTO, SyncSessionDTO, SyncSessionRecordDTO, SyncSessionRoleDTO,
    SyncStatsDTO, SyncStatusDTO, SyncTransportKindDTO, TimelineDirection, TimelineNotesPageDTO,
    TimelineSessionDTO, TimelineSessionsPageDTO,
};
