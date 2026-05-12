use super::*;

impl SynapService {
    pub(crate) fn public_key_info_to_dto(
        id: String,
        algorithm: String,
        public_key: [u8; 32],
    ) -> PublicKeyInfoDTO {
        let fingerprint = crypto::public_key_fingerprint(&public_key);
        PublicKeyInfoDTO {
            id,
            algorithm,
            public_key: public_key.to_vec(),
            fingerprint: fingerprint.to_vec(),
            avatar_png: crypto::generate_public_key_avatar_png(&public_key),
            display_public_key_base64: BASE64_STANDARD.encode(public_key),
            kaomoji_fingerprint: crypto::generate_kaomoji_fingerprint(&fingerprint),
        }
    }

    pub(crate) fn peer_to_dto(record: crypto::TrustedPublicKeyRecord) -> PeerDTO {
        let status = match record.status {
            crate::models::crypto::KeyStatus::Pending => PeerTrustStatusDTO::Pending,
            crate::models::crypto::KeyStatus::Active => PeerTrustStatusDTO::Trusted,
            crate::models::crypto::KeyStatus::Retired => PeerTrustStatusDTO::Retired,
            crate::models::crypto::KeyStatus::Revoked => PeerTrustStatusDTO::Revoked,
        };

        PeerDTO {
            id: record.id.to_string(),
            algorithm: record.algorithm,
            public_key: record.public_key.to_vec(),
            fingerprint: record.fingerprint.to_vec(),
            avatar_png: crypto::generate_public_key_avatar_png(&record.public_key),
            kaomoji_fingerprint: crypto::generate_kaomoji_fingerprint(&record.fingerprint),
            display_public_key_base64: BASE64_STANDARD.encode(record.public_key),
            note: record.note,
            status,
        }
    }

    pub(crate) fn sync_stats_to_dto(stats: crate::sync::SyncStats) -> SyncStatsDTO {
        SyncStatsDTO {
            records_sent: stats.records_sent as u64,
            records_received: stats.records_received as u64,
            records_applied: stats.records_applied as u64,
            records_skipped: stats.records_skipped as u64,
            bytes_sent: stats.bytes_sent as u64,
            bytes_received: stats.bytes_received as u64,
            duration_ms: stats.duration_ms,
        }
    }

    pub(crate) fn sync_stats_record_to_dto(
        record: SyncStatsRecord,
        peer: Option<crypto::TrustedPublicKeyRecord>,
    ) -> SyncSessionRecordDTO {
        let peer_fingerprint = peer
            .as_ref()
            .map(|peer| peer.fingerprint)
            .unwrap_or_else(|| crypto::public_key_fingerprint(&record.peer_public_key));
        let peer_label = peer.as_ref().and_then(|peer| peer.note.clone());
        let (transport, relay_url) = match record.transport {
            SyncTransportKind::Direct => (SyncTransportKindDTO::Direct, None),
            SyncTransportKind::RelayFetch { relay_url } => {
                (SyncTransportKindDTO::RelayFetch, Some(relay_url))
            }
            SyncTransportKind::RelayPush { relay_url } => {
                (SyncTransportKindDTO::RelayPush, Some(relay_url))
            }
        };

        SyncSessionRecordDTO {
            id: record.id.to_string(),
            role: match record.role {
                SyncSessionRole::Initiator => SyncSessionRoleDTO::Initiator,
                SyncSessionRole::Listener => SyncSessionRoleDTO::Listener,
                SyncSessionRole::RelayFetch => SyncSessionRoleDTO::RelayFetch,
                SyncSessionRole::RelayPush => SyncSessionRoleDTO::RelayPush,
            },
            status: match record.status {
                SyncSessionStatus::Completed => SyncStatusDTO::Completed,
                SyncSessionStatus::PendingTrust => SyncStatusDTO::PendingTrust,
                SyncSessionStatus::Failed => SyncStatusDTO::Failed,
            },
            transport,
            relay_url,
            peer_label,
            peer_public_key: record.peer_public_key.to_vec(),
            peer_fingerprint: peer_fingerprint.to_vec(),
            display_peer_fingerprint_base64: BASE64_STANDARD.encode(peer_fingerprint),
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

    pub(crate) fn peer_sync_stats_to_dto(
        record: PeerSyncStatsRecord,
        peer: Option<crypto::TrustedPublicKeyRecord>,
    ) -> PeerSyncStatsDTO {
        let peer_fingerprint = peer
            .as_ref()
            .map(|peer| peer.fingerprint)
            .unwrap_or_else(|| crypto::public_key_fingerprint(&record.peer_public_key));
        let peer_label = peer.as_ref().and_then(|peer| peer.note.clone());
        let peer_status = peer.as_ref().map(|peer| match peer.status {
            crate::models::crypto::KeyStatus::Pending => PeerTrustStatusDTO::Pending,
            crate::models::crypto::KeyStatus::Active => PeerTrustStatusDTO::Trusted,
            crate::models::crypto::KeyStatus::Retired => PeerTrustStatusDTO::Retired,
            crate::models::crypto::KeyStatus::Revoked => PeerTrustStatusDTO::Revoked,
        });

        PeerSyncStatsDTO {
            peer_label,
            peer_public_key: record.peer_public_key.to_vec(),
            peer_fingerprint: peer_fingerprint.to_vec(),
            peer_status,
            recent_sessions: record
                .recent_sessions
                .into_iter()
                .map(|session| Self::sync_stats_record_to_dto(session, peer.clone()))
                .collect(),
        }
    }

    pub(crate) fn share_stats_to_dto(stats: crate::sync::ShareStats) -> ShareStatsDTO {
        ShareStatsDTO {
            records: stats.records as u64,
            records_applied: stats.applied as u64,
            bytes: stats.bytes as u64,
            duration_ms: stats.duration_ms,
        }
    }

    pub(crate) fn relay_fetch_stats_to_dto(
        stats: crate::sync::RelayFetchStats,
    ) -> RelayFetchStatsDTO {
        RelayFetchStatsDTO {
            fetched_messages: stats.fetched_messages,
            imported_messages: stats.imported_messages,
            dropped_untrusted_messages: stats.dropped_untrusted_messages,
            acked_messages: stats.acked_messages,
        }
    }

    pub(crate) fn relay_push_stats_to_dto(stats: crate::sync::RelayPushStats) -> RelayPushStatsDTO {
        RelayPushStatsDTO {
            trusted_peers: stats.trusted_peers,
            posted_messages: stats.posted_messages,
            full_sync_messages: stats.full_sync_messages,
            incremental_sync_messages: stats.incremental_sync_messages,
        }
    }
}
