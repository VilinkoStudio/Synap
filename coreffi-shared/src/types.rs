//! FFI-compatible type conversions for Synap.

use synap_core::dto::{
    LocalIdentityDTO as CoreLocalIdentityDto, NoteBriefDTO as CoreNoteBriefDto,
    NoteContentDiffStatsDTO as CoreNoteContentDiffStatsDto, NoteDTO as CoreNoteDto,
    NoteNeighborContextDTO as CoreNoteNeighborContextDto, NoteNeighborsDTO as CoreNoteNeighborsDto,
    NoteSegmentBranchChoiceDTO as CoreNoteSegmentBranchChoiceDto,
    NoteSegmentDTO as CoreNoteSegmentDto, NoteSegmentDirectionDTO as CoreNoteSegmentDirectionDto,
    NoteSegmentStepDTO as CoreNoteSegmentStepDto, NoteTagDiffDTO as CoreNoteTagDiffDto,
    NoteTextChangeDTO as CoreNoteTextChangeDto, NoteTextChangeKindDTO as CoreNoteTextChangeKindDto,
    NoteVersionDTO as CoreNoteVersionDto, NoteVersionDiffDTO as CoreNoteVersionDiffDto,
    PeerDTO as CorePeerDto, PeerTrustStatusDTO as CorePeerTrustStatusDto,
    PublicKeyInfoDTO as CorePublicKeyInfoDto, RelayFetchStatsDTO as CoreRelayFetchStatsDto,
    RelayPushStatsDTO as CoreRelayPushStatsDto, SearchResultDTO as CoreSearchResultDto,
    SearchSourceDTO as CoreSearchSourceDto, ShareStatsDTO as CoreShareStatsDto,
    StarmapPointDTO as CoreStarmapPointDto, SyncSessionDTO as CoreSyncSessionDto,
    SyncSessionRecordDTO as CoreSyncSessionRecordDto, SyncSessionRoleDTO as CoreSyncSessionRoleDto,
    SyncStatsDTO as CoreSyncStatsDto, SyncStatusDTO as CoreSyncStatusDto,
    SyncTransportKindDTO as CoreSyncTransportKindDto,
    TimelineNotesPageDTO as CoreTimelineNotesPageDto, TimelineSessionDTO as CoreTimelineSessionDto,
    TimelineSessionsPageDTO as CoreTimelineSessionsPageDto,
};
use synap_core::service::FilteredNoteStatus as CoreFilteredNoteStatus;
use synap_core::service::TimelineDirection as CoreTimelineDirection;
use synap_core::BuildInfo as CoreBuildInfo;

/// A note DTO that is friendly to UniFFI/Kotlin consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteBriefDTO {
    pub id: String,
    pub content_preview: String,
    pub created_at: i64,
}

impl From<CoreNoteBriefDto> for NoteBriefDTO {
    fn from(note: CoreNoteBriefDto) -> Self {
        Self {
            id: note.id,
            content_preview: note.content_preview,
            created_at: note.created_at as i64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDTO {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub deleted: bool,
    pub reply_to: Option<NoteBriefDTO>,
    pub edited_from: Option<NoteBriefDTO>,
}

impl From<CoreNoteDto> for NoteDTO {
    fn from(note: CoreNoteDto) -> Self {
        Self {
            id: note.id,
            content: note.content,
            tags: note.tags,
            created_at: note.created_at as i64,
            deleted: note.deleted,
            reply_to: note.reply_to.map(Into::into),
            edited_from: note.edited_from.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSegmentDirectionDTO {
    Forward,
    Backward,
}

impl From<CoreNoteSegmentDirectionDto> for NoteSegmentDirectionDTO {
    fn from(direction: CoreNoteSegmentDirectionDto) -> Self {
        match direction {
            CoreNoteSegmentDirectionDto::Forward => Self::Forward,
            CoreNoteSegmentDirectionDto::Backward => Self::Backward,
        }
    }
}

impl From<NoteSegmentDirectionDTO> for CoreNoteSegmentDirectionDto {
    fn from(direction: NoteSegmentDirectionDTO) -> Self {
        match direction {
            NoteSegmentDirectionDTO::Forward => Self::Forward,
            NoteSegmentDirectionDTO::Backward => Self::Backward,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSegmentBranchChoiceDTO {
    pub note: NoteDTO,
    pub weight: u32,
}

impl From<CoreNoteSegmentBranchChoiceDto> for NoteSegmentBranchChoiceDTO {
    fn from(choice: CoreNoteSegmentBranchChoiceDto) -> Self {
        Self {
            note: choice.note.into(),
            weight: choice.weight,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteNeighborContextDTO {
    pub note: NoteDTO,
    pub weight: u32,
    pub parents: Vec<NoteSegmentBranchChoiceDTO>,
    pub children: Vec<NoteSegmentBranchChoiceDTO>,
}

impl From<CoreNoteNeighborContextDto> for NoteNeighborContextDTO {
    fn from(context: CoreNoteNeighborContextDto) -> Self {
        Self {
            note: context.note.into(),
            weight: context.weight,
            parents: context.parents.into_iter().map(Into::into).collect(),
            children: context.children.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteNeighborsDTO {
    pub note: NoteDTO,
    pub parents: Vec<NoteSegmentBranchChoiceDTO>,
    pub children: Vec<NoteSegmentBranchChoiceDTO>,
    pub parent_contexts: Vec<NoteNeighborContextDTO>,
    pub child_contexts: Vec<NoteNeighborContextDTO>,
}

impl From<CoreNoteNeighborsDto> for NoteNeighborsDTO {
    fn from(neighbors: CoreNoteNeighborsDto) -> Self {
        Self {
            note: neighbors.note.into(),
            parents: neighbors.parents.into_iter().map(Into::into).collect(),
            children: neighbors.children.into_iter().map(Into::into).collect(),
            parent_contexts: neighbors
                .parent_contexts
                .into_iter()
                .map(Into::into)
                .collect(),
            child_contexts: neighbors
                .child_contexts
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSegmentStepDTO {
    pub note: NoteDTO,
    pub next_choices: Vec<NoteSegmentBranchChoiceDTO>,
    pub prev_choices: Vec<NoteSegmentBranchChoiceDTO>,
    pub stops_here: bool,
}

impl From<CoreNoteSegmentStepDto> for NoteSegmentStepDTO {
    fn from(step: CoreNoteSegmentStepDto) -> Self {
        Self {
            note: step.note.into(),
            next_choices: step.next_choices.into_iter().map(Into::into).collect(),
            prev_choices: step.prev_choices.into_iter().map(Into::into).collect(),
            stops_here: step.stops_here,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSegmentDTO {
    pub anchor_id: String,
    pub direction: NoteSegmentDirectionDTO,
    pub steps: Vec<NoteSegmentStepDTO>,
}

impl From<CoreNoteSegmentDto> for NoteSegmentDTO {
    fn from(segment: CoreNoteSegmentDto) -> Self {
        Self {
            anchor_id: segment.anchor_id,
            direction: segment.direction.into(),
            steps: segment.steps.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteTextChangeKindDTO {
    Equal,
    Insert,
    Delete,
}

impl From<CoreNoteTextChangeKindDto> for NoteTextChangeKindDTO {
    fn from(kind: CoreNoteTextChangeKindDto) -> Self {
        match kind {
            CoreNoteTextChangeKindDto::Equal => Self::Equal,
            CoreNoteTextChangeKindDto::Insert => Self::Insert,
            CoreNoteTextChangeKindDto::Delete => Self::Delete,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTextChangeDTO {
    pub kind: NoteTextChangeKindDTO,
    pub value: String,
}

impl From<CoreNoteTextChangeDto> for NoteTextChangeDTO {
    fn from(change: CoreNoteTextChangeDto) -> Self {
        Self {
            kind: change.kind.into(),
            value: change.value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTagDiffDTO {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl From<CoreNoteTagDiffDto> for NoteTagDiffDTO {
    fn from(diff: CoreNoteTagDiffDto) -> Self {
        Self {
            added: diff.added,
            removed: diff.removed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteContentDiffStatsDTO {
    pub inserted_chars: u32,
    pub deleted_chars: u32,
    pub inserted_lines: u32,
    pub deleted_lines: u32,
}

impl From<CoreNoteContentDiffStatsDto> for NoteContentDiffStatsDTO {
    fn from(stats: CoreNoteContentDiffStatsDto) -> Self {
        Self {
            inserted_chars: stats.inserted_chars,
            deleted_chars: stats.deleted_chars,
            inserted_lines: stats.inserted_lines,
            deleted_lines: stats.deleted_lines,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteVersionDiffDTO {
    pub tags: NoteTagDiffDTO,
    pub content: Vec<NoteTextChangeDTO>,
    pub content_summary: Vec<NoteTextChangeDTO>,
    pub content_stats: NoteContentDiffStatsDTO,
}

impl From<CoreNoteVersionDiffDto> for NoteVersionDiffDTO {
    fn from(diff: CoreNoteVersionDiffDto) -> Self {
        Self {
            tags: diff.tags.into(),
            content: diff.content.into_iter().map(Into::into).collect(),
            content_summary: diff.content_summary.into_iter().map(Into::into).collect(),
            content_stats: diff.content_stats.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteVersionDTO {
    pub note: NoteDTO,
    pub diff: NoteVersionDiffDTO,
}

impl From<CoreNoteVersionDto> for NoteVersionDTO {
    fn from(version: CoreNoteVersionDto) -> Self {
        Self {
            note: version.note.into(),
            diff: version.diff.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSourceDTO {
    Fuzzy,
    Semantic,
}

impl From<CoreSearchSourceDto> for SearchSourceDTO {
    fn from(source: CoreSearchSourceDto) -> Self {
        match source {
            CoreSearchSourceDto::Fuzzy => Self::Fuzzy,
            CoreSearchSourceDto::Semantic => Self::Semantic,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResultDTO {
    pub note: NoteDTO,
    pub score: f32,
    pub sources: Vec<SearchSourceDTO>,
}

impl From<CoreSearchResultDto> for SearchResultDTO {
    fn from(result: CoreSearchResultDto) -> Self {
        Self {
            note: result.note.into(),
            score: result.score,
            sources: result.sources.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineNotesPageDTO {
    pub notes: Vec<NoteDTO>,
    pub next_cursor: Option<String>,
}

impl From<CoreTimelineNotesPageDto> for TimelineNotesPageDTO {
    fn from(page: CoreTimelineNotesPageDto) -> Self {
        Self {
            notes: page.notes.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSessionDTO {
    pub started_at: i64,
    pub ended_at: i64,
    pub note_count: u32,
    pub notes: Vec<NoteDTO>,
}

impl From<CoreTimelineSessionDto> for TimelineSessionDTO {
    fn from(session: CoreTimelineSessionDto) -> Self {
        Self {
            started_at: session.started_at as i64,
            ended_at: session.ended_at as i64,
            note_count: session.note_count,
            notes: session.notes.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSessionsPageDTO {
    pub sessions: Vec<TimelineSessionDTO>,
    pub next_cursor: Option<String>,
}

impl From<CoreTimelineSessionsPageDto> for TimelineSessionsPageDTO {
    fn from(page: CoreTimelineSessionsPageDto) -> Self {
        Self {
            sessions: page.sessions.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StarmapPointDTO {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

impl From<CoreStarmapPointDto> for StarmapPointDTO {
    fn from(point: CoreStarmapPointDto) -> Self {
        Self {
            id: point.id,
            x: point.x,
            y: point.y,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareStatsDTO {
    pub records: u64,
    pub records_applied: u64,
    pub bytes: u64,
    pub duration_ms: u64,
}

impl From<CoreShareStatsDto> for ShareStatsDTO {
    fn from(stats: CoreShareStatsDto) -> Self {
        Self {
            records: stats.records,
            records_applied: stats.records_applied,
            bytes: stats.bytes,
            duration_ms: stats.duration_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatusDTO {
    Completed,
    PendingTrust,
    Failed,
}

impl From<CoreSyncStatusDto> for SyncStatusDTO {
    fn from(status: CoreSyncStatusDto) -> Self {
        match status {
            CoreSyncStatusDto::Completed => Self::Completed,
            CoreSyncStatusDto::PendingTrust => Self::PendingTrust,
            CoreSyncStatusDto::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatsDTO {
    pub records_sent: u64,
    pub records_received: u64,
    pub records_applied: u64,
    pub records_skipped: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
}

impl From<CoreSyncStatsDto> for SyncStatsDTO {
    fn from(stats: CoreSyncStatsDto) -> Self {
        Self {
            records_sent: stats.records_sent,
            records_received: stats.records_received,
            records_applied: stats.records_applied,
            records_skipped: stats.records_skipped,
            bytes_sent: stats.bytes_sent,
            bytes_received: stats.bytes_received,
            duration_ms: stats.duration_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayFetchStatsDTO {
    pub fetched_messages: u64,
    pub imported_messages: u64,
    pub dropped_untrusted_messages: u64,
    pub acked_messages: u64,
}

impl From<CoreRelayFetchStatsDto> for RelayFetchStatsDTO {
    fn from(stats: CoreRelayFetchStatsDto) -> Self {
        Self {
            fetched_messages: stats.fetched_messages,
            imported_messages: stats.imported_messages,
            dropped_untrusted_messages: stats.dropped_untrusted_messages,
            acked_messages: stats.acked_messages,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayPushStatsDTO {
    pub trusted_peers: u64,
    pub posted_messages: u64,
    pub full_sync_messages: u64,
    pub incremental_sync_messages: u64,
}

impl From<CoreRelayPushStatsDto> for RelayPushStatsDTO {
    fn from(stats: CoreRelayPushStatsDto) -> Self {
        Self {
            trusted_peers: stats.trusted_peers,
            posted_messages: stats.posted_messages,
            full_sync_messages: stats.full_sync_messages,
            incremental_sync_messages: stats.incremental_sync_messages,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSessionRoleDTO {
    Initiator,
    Listener,
    RelayFetch,
    RelayPush,
}

impl From<CoreSyncSessionRoleDto> for SyncSessionRoleDTO {
    fn from(role: CoreSyncSessionRoleDto) -> Self {
        match role {
            CoreSyncSessionRoleDto::Initiator => Self::Initiator,
            CoreSyncSessionRoleDto::Listener => Self::Listener,
            CoreSyncSessionRoleDto::RelayFetch => Self::RelayFetch,
            CoreSyncSessionRoleDto::RelayPush => Self::RelayPush,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTransportKindDTO {
    Direct,
    RelayFetch,
    RelayPush,
}

impl From<CoreSyncTransportKindDto> for SyncTransportKindDTO {
    fn from(kind: CoreSyncTransportKindDto) -> Self {
        match kind {
            CoreSyncTransportKindDto::Direct => Self::Direct,
            CoreSyncTransportKindDto::RelayFetch => Self::RelayFetch,
            CoreSyncTransportKindDto::RelayPush => Self::RelayPush,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerTrustStatusDTO {
    Pending,
    Trusted,
    Retired,
    Revoked,
}

impl From<CorePeerTrustStatusDto> for PeerTrustStatusDTO {
    fn from(status: CorePeerTrustStatusDto) -> Self {
        match status {
            CorePeerTrustStatusDto::Pending => Self::Pending,
            CorePeerTrustStatusDto::Trusted => Self::Trusted,
            CorePeerTrustStatusDto::Retired => Self::Retired,
            CorePeerTrustStatusDto::Revoked => Self::Revoked,
        }
    }
}

impl From<PeerTrustStatusDTO> for CorePeerTrustStatusDto {
    fn from(status: PeerTrustStatusDTO) -> Self {
        match status {
            PeerTrustStatusDTO::Pending => Self::Pending,
            PeerTrustStatusDTO::Trusted => Self::Trusted,
            PeerTrustStatusDTO::Retired => Self::Retired,
            PeerTrustStatusDTO::Revoked => Self::Revoked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKeyInfoDTO {
    pub id: String,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub avatar_png: Vec<u8>,
    pub display_public_key_base64: String,
    pub kaomoji_fingerprint: String,
}

impl From<CorePublicKeyInfoDto> for PublicKeyInfoDTO {
    fn from(info: CorePublicKeyInfoDto) -> Self {
        Self {
            id: info.id,
            algorithm: info.algorithm,
            public_key: info.public_key,
            fingerprint: info.fingerprint,
            avatar_png: info.avatar_png,
            display_public_key_base64: info.display_public_key_base64,
            kaomoji_fingerprint: info.kaomoji_fingerprint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalIdentityDTO {
    pub identity: PublicKeyInfoDTO,
    pub signing: PublicKeyInfoDTO,
}

impl From<CoreLocalIdentityDto> for LocalIdentityDTO {
    fn from(identity: CoreLocalIdentityDto) -> Self {
        Self {
            identity: identity.identity.into(),
            signing: identity.signing.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerDTO {
    pub id: String,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub avatar_png: Vec<u8>,
    pub kaomoji_fingerprint: String,
    pub display_public_key_base64: String,
    pub note: Option<String>,
    pub status: PeerTrustStatusDTO,
}

impl From<CorePeerDto> for PeerDTO {
    fn from(peer: CorePeerDto) -> Self {
        Self {
            id: peer.id,
            algorithm: peer.algorithm,
            public_key: peer.public_key,
            fingerprint: peer.fingerprint,
            avatar_png: peer.avatar_png,
            kaomoji_fingerprint: peer.kaomoji_fingerprint,
            display_public_key_base64: peer.display_public_key_base64,
            note: peer.note,
            status: peer.status.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSessionDTO {
    pub status: SyncStatusDTO,
    pub peer: PeerDTO,
    pub stats: Option<SyncStatsDTO>,
}

impl From<CoreSyncSessionDto> for SyncSessionDTO {
    fn from(session: CoreSyncSessionDto) -> Self {
        Self {
            status: session.status.into(),
            peer: session.peer.into(),
            stats: session.stats.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSessionRecordDTO {
    pub id: String,
    pub role: SyncSessionRoleDTO,
    pub status: SyncStatusDTO,
    pub transport: SyncTransportKindDTO,
    pub relay_url: Option<String>,
    pub peer_label: Option<String>,
    pub peer_public_key: Vec<u8>,
    pub peer_fingerprint: Vec<u8>,
    pub display_peer_fingerprint_base64: String,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub records_sent: u64,
    pub records_received: u64,
    pub records_applied: u64,
    pub records_skipped: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

impl From<CoreSyncSessionRecordDto> for SyncSessionRecordDTO {
    fn from(record: CoreSyncSessionRecordDto) -> Self {
        Self {
            id: record.id,
            role: record.role.into(),
            status: record.status.into(),
            transport: record.transport.into(),
            relay_url: record.relay_url,
            peer_label: record.peer_label,
            peer_public_key: record.peer_public_key,
            peer_fingerprint: record.peer_fingerprint,
            display_peer_fingerprint_base64: record.display_peer_fingerprint_base64,
            started_at_ms: record.started_at_ms,
            finished_at_ms: record.finished_at_ms,
            records_sent: record.records_sent,
            records_received: record.records_received,
            records_applied: record.records_applied,
            records_skipped: record.records_skipped,
            bytes_sent: record.bytes_sent,
            bytes_received: record.bytes_received,
            duration_ms: record.duration_ms,
            error_message: record.error_message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    pub crate_version: String,
    pub git_branch: String,
    pub git_commit: String,
    pub git_short_commit: String,
    pub git_tag: Option<String>,
    pub display_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteredNoteStatus {
    All,
    Normal,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineDirection {
    Older,
    Newer,
}

impl From<FilteredNoteStatus> for CoreFilteredNoteStatus {
    fn from(status: FilteredNoteStatus) -> Self {
        match status {
            FilteredNoteStatus::All => CoreFilteredNoteStatus::All,
            FilteredNoteStatus::Normal => CoreFilteredNoteStatus::Normal,
            FilteredNoteStatus::Deleted => CoreFilteredNoteStatus::Deleted,
        }
    }
}

impl From<TimelineDirection> for CoreTimelineDirection {
    fn from(direction: TimelineDirection) -> Self {
        match direction {
            TimelineDirection::Older => CoreTimelineDirection::Older,
            TimelineDirection::Newer => CoreTimelineDirection::Newer,
        }
    }
}

impl From<CoreBuildInfo> for BuildInfo {
    fn from(info: CoreBuildInfo) -> Self {
        let display_version = info.display_version();
        Self {
            crate_version: info.crate_version.to_string(),
            git_branch: info.git_branch.to_string(),
            git_commit: info.git_commit.to_string(),
            git_short_commit: info.git_short_commit.to_string(),
            git_tag: info.git_tag.map(ToString::to_string),
            display_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_dto_conversion() {
        let core_note = CoreNoteDto {
            id: "0195f9a8-d085-7f9d-a604-469e0f91d0e3".to_string(),
            content: "Test content".to_string(),
            tags: vec!["rust".to_string(), "android".to_string()],
            created_at: 1_742_165_200_000,
            deleted: false,
            reply_to: Some(CoreNoteBriefDto {
                id: "0195f9a8-d085-7f9d-a604-469e0f91d0e4".to_string(),
                content_preview: "Parent preview".to_string(),
                created_at: 1_742_165_100_000,
            }),
            edited_from: None,
        };

        let ffi_note: NoteDTO = core_note.into();
        assert_eq!(ffi_note.id, "0195f9a8-d085-7f9d-a604-469e0f91d0e3");
        assert_eq!(ffi_note.content, "Test content");
        assert_eq!(ffi_note.tags, vec!["rust", "android"]);
        assert_eq!(ffi_note.created_at, 1_742_165_200_000);
        assert!(!ffi_note.deleted);
        assert_eq!(
            ffi_note
                .reply_to
                .as_ref()
                .map(|brief| brief.content_preview.as_str()),
            Some("Parent preview")
        );
        assert!(ffi_note.edited_from.is_none());
    }

    #[test]
    fn test_build_info_conversion() {
        let core_info = CoreBuildInfo {
            crate_version: "0.1.0",
            git_branch: "main",
            git_commit: "abcdef0123456789",
            git_short_commit: "abcdef012345",
            git_tag: Some("v0.1.0"),
        };

        let ffi_info: BuildInfo = core_info.into();
        assert_eq!(ffi_info.crate_version, "0.1.0");
        assert_eq!(ffi_info.git_branch, "main");
        assert_eq!(ffi_info.git_commit, "abcdef0123456789");
        assert_eq!(ffi_info.git_short_commit, "abcdef012345");
        assert_eq!(ffi_info.git_tag.as_deref(), Some("v0.1.0"));
        assert_eq!(ffi_info.display_version, "v0.1.0 (abcdef012345)");
    }

    #[test]
    fn test_filtered_note_status_conversion() {
        assert_eq!(
            CoreFilteredNoteStatus::from(FilteredNoteStatus::All),
            CoreFilteredNoteStatus::All
        );
        assert_eq!(
            CoreFilteredNoteStatus::from(FilteredNoteStatus::Normal),
            CoreFilteredNoteStatus::Normal
        );
        assert_eq!(
            CoreFilteredNoteStatus::from(FilteredNoteStatus::Deleted),
            CoreFilteredNoteStatus::Deleted
        );
    }

    #[test]
    fn test_peer_trust_status_conversion() {
        assert_eq!(
            PeerTrustStatusDTO::from(CorePeerTrustStatusDto::Pending),
            PeerTrustStatusDTO::Pending
        );
        assert_eq!(
            PeerTrustStatusDTO::from(CorePeerTrustStatusDto::Trusted),
            PeerTrustStatusDTO::Trusted
        );
        assert_eq!(
            PeerTrustStatusDTO::from(CorePeerTrustStatusDto::Retired),
            PeerTrustStatusDTO::Retired
        );
        assert_eq!(
            PeerTrustStatusDTO::from(CorePeerTrustStatusDto::Revoked),
            PeerTrustStatusDTO::Revoked
        );
    }
}
