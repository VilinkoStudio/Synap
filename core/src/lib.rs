#[cfg(not(target_arch = "wasm32"))]
pub mod crypto;
#[cfg(not(target_arch = "wasm32"))]
mod db;
#[cfg(not(target_arch = "wasm32"))]
mod models;
#[cfg(not(target_arch = "wasm32"))]
mod text;
#[cfg(not(target_arch = "wasm32"))]
mod views;

// Public API modules
#[cfg(not(target_arch = "wasm32"))]
pub mod envelope;
#[cfg(not(target_arch = "wasm32"))]
pub mod nlp;
#[cfg(not(target_arch = "wasm32"))]
pub mod service;
#[cfg(not(target_arch = "wasm32"))]
pub mod sync;

pub mod dto;
#[cfg(not(target_arch = "wasm32"))]
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod search;
pub mod version;
/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use dto::{
    EmbeddingBackfillProgressDTO, EmbeddingConfigDTO, EmbeddingProviderDTO, LocalIdentityDTO,
    NoteContentDiffStatsDTO, NoteDTO, NoteDraftDTO, NoteNeighborContextDTO, NoteNeighborsDTO,
    NoteSegmentBranchChoiceDTO, NoteSegmentDTO, NoteSegmentDirectionDTO, NoteSegmentStepDTO,
    NoteTagDiffDTO, NoteTextChangeDTO, NoteTextChangeKindDTO, NoteVersionDTO, NoteVersionDiffDTO,
    PeerDTO, PeerSyncStatsDTO, PeerTrustStatusDTO, PublicKeyInfoDTO, RelayFetchStatsDTO,
    RelayPushStatsDTO, SearchResultDTO, SearchSourceDTO, ShareStatsDTO, SyncSessionDTO,
    SyncSessionRecordDTO, SyncSessionRoleDTO, SyncStatsDTO, SyncStatusDTO, SyncTransportKindDTO,
    TimelineGroupDTO, TimelineNotesPageDTO, TimelineSessionDTO, TimelineSessionsPageDTO,
};
#[cfg(not(target_arch = "wasm32"))]
pub use error::{NoteError, ServiceError};
#[cfg(not(target_arch = "wasm32"))]
pub use models::config::{ConfigError, CoreConfig, EmbeddingConfig};
#[cfg(not(target_arch = "wasm32"))]
pub use service::{SynapService, TimelineDirection};
pub use version::{build_info, version_string, BuildInfo};
