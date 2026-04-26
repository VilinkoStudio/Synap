use serde::{Deserialize, Serialize};

/// 绝对纯净的、跨端通用的 DTO
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")] // 照顾 TS 和 Kotlin 的命名习惯
pub struct NoteDTO {
    pub id: String, // Uuid 转成标准的 36 位字符串
    // pub short_id: String, // 8位 NanoID
    pub content: String,
    pub tags: Vec<String>, // 直接给文字，前端不关心 Tag 的内部 UUID
    pub created_at: u64,   // 毫秒时间戳
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineNotesPageDTO {
    pub notes: Vec<NoteDTO>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSessionDTO {
    pub started_at: u64,
    pub ended_at: u64,
    pub note_count: u32,
    pub notes: Vec<NoteDTO>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSessionsPageDTO {
    pub sessions: Vec<TimelineSessionDTO>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StarmapPointDTO {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PeerTrustStatusDTO {
    Pending,
    Trusted,
    Retired,
    Revoked,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyInfoDTO {
    pub id: String,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub display_public_key_base64: String,
    pub kaomoji_fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalIdentityDTO {
    pub identity: PublicKeyInfoDTO,
    pub signing: PublicKeyInfoDTO,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeerDTO {
    pub id: String,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub kaomoji_fingerprint: String,
    pub note: Option<String>,
    pub status: PeerTrustStatusDTO,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatsDTO {
    pub records_sent: u64,
    pub records_received: u64,
    pub records_applied: u64,
    pub records_skipped: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareStatsDTO {
    pub records: u64,
    pub records_applied: u64,
    pub bytes: u64,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncStatusDTO {
    Completed,
    PendingTrust,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncSessionDTO {
    pub status: SyncStatusDTO,
    pub peer: PeerDTO,
    pub stats: Option<SyncStatsDTO>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncSessionRoleDTO {
    Initiator,
    Listener,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncSessionRecordDTO {
    pub id: String,
    pub role: SyncSessionRoleDTO,
    pub status: SyncStatusDTO,
    pub peer_label: Option<String>,
    pub peer_public_key: Option<Vec<u8>>,
    pub peer_fingerprint: Option<Vec<u8>>,
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
