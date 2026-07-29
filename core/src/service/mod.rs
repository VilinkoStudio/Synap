use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

mod convert;
pub mod discovery;
pub use discovery::MdnsDiscoveryError;
mod embedding;
mod note_command;
mod note_draft;

mod note_query;
mod peer;
mod relay_peer;
mod search;
mod support;
mod sync;
#[cfg(test)]
mod tests;

use crate::{
    crypto,
    dto::{
        EmbeddingBackfillProgressDTO, EmbeddingConfigDTO, EmbeddingProviderDTO, LocalIdentityDTO,
        NoteDTO, NoteNeighborsDTO, NoteSegmentDTO, NoteSegmentDirectionDTO, NoteVersionDTO,
        PeerDTO, PeerSyncStatsDTO, PeerTrustStatusDTO, PublicKeyInfoDTO, RelayFetchStatsDTO,
        RelayPushStatsDTO, SearchResultDTO, SearchSourceDTO, ShareStatsDTO,
        SyncSessionDTO, SyncSessionRecordDTO, SyncSessionRoleDTO, SyncStatsDTO, SyncStatusDTO,
        SyncTransportKindDTO, TimelineDensityPointDTO, TimelineGroupDTO, TimelineNotesPageDTO,
        TimelineSessionDTO, TimelineSessionsPageDTO,
    },
    error::ServiceError,
    models::{
        config::{ConfigWriter, CoreConfig},
        crypto::{CryptoReader, CryptoWriter},
        embedding_cache::EmbeddingCacheMetadata,
        note::{Note, NoteReader, NoteRef},
        relay_peer::{RelayPeerReader, RelayPeerRecord, RelayPeerWriter},
        sync_stats::{
            PeerSyncStatsRecord, SyncSessionRole, SyncSessionStatus, SyncStatsReader,
            SyncStatsRecord, SyncStatsWriter, SyncTransportKind,
        },
        tag::{Tag, TagReader, TagWriter},
        tag_profile::TagProfileStore,
    },
    nlp::tag::TagProfileIndex,
    search::{searcher::FuzzyIndex, semantic::SemanticIndex, types::Searchable},
    sync::{RelayInventory, RelaySyncService, ShareService, SyncPeerIdentity, SyncService},
    views::{
        note_segment_view::{NoteSegmentDirection, NoteSegmentView},
        note_version_view::NoteVersionView,
        note_view::NoteView,
        timeline_view::{SessionDetectionConfig, SessionSpan, TimelineView},
    },
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use redb::{Database, ReadTransaction, ReadableDatabase, WriteTransaction};
use std::ops::Bound;
use tempfile::NamedTempFile;
use uuid::{Builder, Uuid};

#[derive(Debug, Default)]
struct ServiceTagRecommender {
    index: RwLock<TagProfileIndex>,
}

impl ServiceTagRecommender {
    fn new() -> Self {
        Self::default()
    }

    fn replace_if_newer(&self, index: TagProfileIndex) -> bool {
        let mut current = self.index.write().expect("tag profile index lock");
        if current.embedding_space() == index.embedding_space()
            && current.generation() > index.generation()
        {
            return false;
        }
        *current = index;
        true
    }

    fn clear(&self) {
        *self.index.write().expect("tag profile index lock") = TagProfileIndex::default();
    }

    fn recommend_tags(&self, query: &[f32], limit: usize) -> Vec<String> {
        self.index
            .read()
            .expect("tag profile index lock")
            .recommend_tags(query, limit)
    }
}

pub struct SynapService {
    db: redb::Database,
    /// Coordinates model use with atomic configuration changes.
    embedding_lifecycle: RwLock<()>,
    /// 强类型核心配置；落盘由 config model 负责，运行时归 service 所有。
    config: Mutex<CoreConfig>,
    #[allow(dead_code)]
    tag_searcher: FuzzyIndex<Tag>,
    #[allow(dead_code)]
    note_searcher: FuzzyIndex<Note>,
    semantic_index: SemanticIndex,
    tag_recommender: ServiceTagRecommender,
    /// Process-local memory drafts. Persisted drafts live in their own unindexed redb table.
    draft_store: Mutex<crate::models::note_draft::NoteDraftMemoryStore>,
    /// Serializes claim/receipt transitions so one draft can append at most once.
    draft_commit_lock: Mutex<()>,
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

const DEFAULT_SESSION_DETECTION_CONFIG: SessionDetectionConfig =
    SessionDetectionConfig::new(5 * 60 * 1000);

#[derive(Debug, Clone, Copy)]
enum ServiceSyncRole {
    Initiator,
    Listener,
}
