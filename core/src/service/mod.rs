use std::{
    collections::HashSet,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};

mod convert;
mod note_command;
mod note_query;
mod peer;
mod search;
mod support;
mod sync;
#[cfg(test)]
mod tests;

use crate::{
    crypto,
    dto::{
        LocalIdentityDTO, NoteDTO, PeerDTO, PeerTrustStatusDTO, PublicKeyInfoDTO, ShareStatsDTO,
        SyncSessionDTO, SyncSessionRecordDTO, SyncSessionRoleDTO, SyncStatsDTO, SyncStatusDTO,
        TimelineNotesPageDTO, TimelineSessionDTO, TimelineSessionsPageDTO,
    },
    error::ServiceError,
    models::{
        crypto::{CryptoReader, CryptoWriter},
        note::{Note, NoteReader, NoteRef},
        sync_stats::{
            SyncSessionRole, SyncSessionStatus, SyncStatsReader, SyncStatsRecord, SyncStatsWriter,
        },
        tag::{Tag, TagReader, TagWriter},
    },
    nlp::{NlpDocument, NlpTagIndex},
    search::searcher::FuzzyIndex,
    sync::{ShareService, SyncPeerIdentity, SyncService},
    views::{
        note_view::NoteView,
        timeline_view::{SessionDetectionConfig, SessionSpan, TimelinePoint, TimelineView},
    },
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use redb::{Database, ReadTransaction, ReadableDatabase, WriteTransaction};
use std::ops::Bound;
use tempfile::NamedTempFile;
use uuid::Uuid;

#[derive(Debug, Default)]
struct ServiceTagRecommender {
    index: Mutex<NlpTagIndex>,
}

impl ServiceTagRecommender {
    fn new() -> Self {
        Self::default()
    }

    fn rebuild(&self, docs: Vec<NlpDocument>) {
        self.index.lock().unwrap().build(docs);
    }

    fn recommend_tag(&self, content: &str, limit: usize) -> Vec<String> {
        self.index.lock().unwrap().recommend_tag(content, limit)
    }
}

pub struct SynapService {
    db: redb::Database,
    #[allow(dead_code)]
    tag_searcher: FuzzyIndex<Tag>,
    #[allow(dead_code)]
    note_searcher: FuzzyIndex<Note>,
    tag_recommender: ServiceTagRecommender,
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
