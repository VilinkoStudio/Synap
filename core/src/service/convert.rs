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

    pub(crate) fn sync_stats_record_to_dto(record: SyncStatsRecord) -> SyncSessionRecordDTO {
        SyncSessionRecordDTO {
            id: record.id.to_string(),
            role: match record.role {
                SyncSessionRole::Initiator => SyncSessionRoleDTO::Initiator,
                SyncSessionRole::Listener => SyncSessionRoleDTO::Listener,
            },
            status: match record.status {
                SyncSessionStatus::Completed => SyncStatusDTO::Completed,
                SyncSessionStatus::PendingTrust => SyncStatusDTO::PendingTrust,
                SyncSessionStatus::Failed => SyncStatusDTO::Failed,
            },
            peer_label: record.peer_label,
            peer_public_key: record.peer_public_key,
            peer_fingerprint: record.peer_fingerprint,
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

    pub(crate) fn share_stats_to_dto(stats: crate::sync::ShareStats) -> ShareStatsDTO {
        ShareStatsDTO {
            records: stats.records as u64,
            records_applied: stats.applied as u64,
            bytes: stats.bytes as u64,
            duration_ms: stats.duration_ms,
        }
    }
}
