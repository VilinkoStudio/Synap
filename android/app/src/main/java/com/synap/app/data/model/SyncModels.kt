package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.LocalIdentityDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.PeerDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.PeerTrustStatusDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.PublicKeyInfoDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.RelayFetchStatsDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.RelayPushStatsDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncSessionDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncSessionRecordDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncSessionRoleDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncStatsDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncStatusDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncTransportKindDto

data class PublicKeyInfo(
    val id: String,
    val algorithm: String,
    val publicKey: ByteArray,
    val fingerprint: ByteArray,
    val avatarPng: ByteArray,
    val displayPublicKeyBase64: String,
    val kaomojiFingerprint: String,
)

data class LocalIdentity(
    val identity: PublicKeyInfo,
    val signing: PublicKeyInfo,
)

enum class PeerTrustStatus {
    Pending,
    Trusted,
    Retired,
    Revoked,
}

data class PeerRecord(
    val id: String,
    val algorithm: String,
    val publicKey: ByteArray,
    val fingerprint: ByteArray,
    val avatarPng: ByteArray,
    val kaomojiFingerprint: String,
    val displayPublicKeyBase64: String,
    val note: String?,
    val status: PeerTrustStatus,
)

enum class SyncConnectionStatus {
    Idle,
    Connecting,
    AwaitingTrust,
    Connected,
    Failed,
}

data class SyncConnectionRecord(
    val id: String,
    val name: String,
    val host: String,
    val port: Int,
    val status: SyncConnectionStatus = SyncConnectionStatus.Idle,
    val statusMessage: String = "已保存，尚未配对",
)

data class DiscoveredSyncPeer(
    val serviceName: String,
    val displayName: String,
    val host: String,
    val port: Int,
    val lastSeenAtMs: Long,
)

data class SyncListenerState(
    val protocol: String = "TCP",
    val backend: String = "Java Socket",
    val isListening: Boolean = false,
    val listenPort: Int? = null,
    val localAddresses: List<String> = emptyList(),
    val status: String = "未启动",
    val errorMessage: String? = null,
)

enum class SyncStatus {
    Completed,
    PendingTrust,
    Failed,
}

enum class SyncSessionRole {
    Initiator,
    Listener,
    RelayFetch,
    RelayPush,
}

enum class SyncTransportKind {
    Direct,
    RelayFetch,
    RelayPush,
}

data class SyncStats(
    val recordsSent: ULong,
    val recordsReceived: ULong,
    val recordsApplied: ULong,
    val recordsSkipped: ULong,
    val bytesSent: ULong,
    val bytesReceived: ULong,
    val durationMs: ULong,
)

data class SyncSession(
    val status: SyncStatus,
    val peer: PeerRecord,
    val stats: SyncStats?,
)

data class SyncSessionRecord(
    val id: String,
    val role: SyncSessionRole,
    val status: SyncStatus,
    val transport: SyncTransportKind,
    val relayUrl: String?,
    val peerLabel: String?,
    val peerPublicKey: ByteArray,
    val peerFingerprint: ByteArray,
    val displayPeerFingerprintBase64: String,
    val startedAtMs: ULong,
    val finishedAtMs: ULong,
    val recordsSent: ULong,
    val recordsReceived: ULong,
    val recordsApplied: ULong,
    val recordsSkipped: ULong,
    val bytesSent: ULong,
    val bytesReceived: ULong,
    val durationMs: ULong,
    val errorMessage: String?,
)

data class RelayFetchStats(
    val fetchedMessages: ULong,
    val importedMessages: ULong,
    val droppedUntrustedMessages: ULong,
    val ackedMessages: ULong,
)

data class RelayPushStats(
    val trustedPeers: ULong,
    val postedMessages: ULong,
    val fullSyncMessages: ULong,
    val incrementalSyncMessages: ULong,
)

internal fun PublicKeyInfoDto.toPublicKeyInfo(): PublicKeyInfo = PublicKeyInfo(
    id = id,
    algorithm = algorithm,
    publicKey = publicKey,
    fingerprint = fingerprint,
    avatarPng = avatarPng,
    displayPublicKeyBase64 = displayPublicKeyBase64,
    kaomojiFingerprint = kaomojiFingerprint,
)

internal fun LocalIdentityDto.toLocalIdentity(): LocalIdentity = LocalIdentity(
    identity = identity.toPublicKeyInfo(),
    signing = signing.toPublicKeyInfo(),
)

internal fun PeerTrustStatusDto.toPeerTrustStatus(): PeerTrustStatus = when (this) {
    PeerTrustStatusDto.PENDING -> PeerTrustStatus.Pending
    PeerTrustStatusDto.TRUSTED -> PeerTrustStatus.Trusted
    PeerTrustStatusDto.RETIRED -> PeerTrustStatus.Retired
    PeerTrustStatusDto.REVOKED -> PeerTrustStatus.Revoked
}

internal fun PeerTrustStatus.toDto(): PeerTrustStatusDto = when (this) {
    PeerTrustStatus.Pending -> PeerTrustStatusDto.PENDING
    PeerTrustStatus.Trusted -> PeerTrustStatusDto.TRUSTED
    PeerTrustStatus.Retired -> PeerTrustStatusDto.RETIRED
    PeerTrustStatus.Revoked -> PeerTrustStatusDto.REVOKED
}

internal fun PeerDto.toPeerRecord(): PeerRecord = PeerRecord(
    id = id,
    algorithm = algorithm,
    publicKey = publicKey,
    fingerprint = fingerprint,
    avatarPng = avatarPng,
    kaomojiFingerprint = kaomojiFingerprint,
    displayPublicKeyBase64 = displayPublicKeyBase64,
    note = note,
    status = status.toPeerTrustStatus(),
)

internal fun List<PeerDto>.toPeerRecords(): List<PeerRecord> = map(PeerDto::toPeerRecord)

internal fun SyncStatusDto.toSyncStatus(): SyncStatus = when (this) {
    SyncStatusDto.COMPLETED -> SyncStatus.Completed
    SyncStatusDto.PENDING_TRUST -> SyncStatus.PendingTrust
    SyncStatusDto.FAILED -> SyncStatus.Failed
}

internal fun SyncSessionRoleDto.toSyncSessionRole(): SyncSessionRole = when (this) {
    SyncSessionRoleDto.INITIATOR -> SyncSessionRole.Initiator
    SyncSessionRoleDto.LISTENER -> SyncSessionRole.Listener
    SyncSessionRoleDto.RELAY_FETCH -> SyncSessionRole.RelayFetch
    SyncSessionRoleDto.RELAY_PUSH -> SyncSessionRole.RelayPush
}

internal fun SyncTransportKindDto.toSyncTransportKind(): SyncTransportKind = when (this) {
    SyncTransportKindDto.DIRECT -> SyncTransportKind.Direct
    SyncTransportKindDto.RELAY_FETCH -> SyncTransportKind.RelayFetch
    SyncTransportKindDto.RELAY_PUSH -> SyncTransportKind.RelayPush
}

internal fun SyncStatsDto.toSyncStats(): SyncStats = SyncStats(
    recordsSent = recordsSent,
    recordsReceived = recordsReceived,
    recordsApplied = recordsApplied,
    recordsSkipped = recordsSkipped,
    bytesSent = bytesSent,
    bytesReceived = bytesReceived,
    durationMs = durationMs,
)

internal fun SyncSessionDto.toSyncSession(): SyncSession = SyncSession(
    status = status.toSyncStatus(),
    peer = peer.toPeerRecord(),
    stats = stats?.toSyncStats(),
)

internal fun SyncSessionRecordDto.toSyncSessionRecord(): SyncSessionRecord = SyncSessionRecord(
    id = id,
    role = role.toSyncSessionRole(),
    status = status.toSyncStatus(),
    transport = transport.toSyncTransportKind(),
    relayUrl = relayUrl,
    peerLabel = peerLabel,
    peerPublicKey = peerPublicKey,
    peerFingerprint = peerFingerprint,
    displayPeerFingerprintBase64 = displayPeerFingerprintBase64,
    startedAtMs = startedAtMs,
    finishedAtMs = finishedAtMs,
    recordsSent = recordsSent,
    recordsReceived = recordsReceived,
    recordsApplied = recordsApplied,
    recordsSkipped = recordsSkipped,
    bytesSent = bytesSent,
    bytesReceived = bytesReceived,
    durationMs = durationMs,
    errorMessage = errorMessage,
)

internal fun List<SyncSessionRecordDto>.toSyncSessionRecords(): List<SyncSessionRecord> =
    map(SyncSessionRecordDto::toSyncSessionRecord)

internal fun RelayFetchStatsDto.toRelayFetchStats(): RelayFetchStats = RelayFetchStats(
    fetchedMessages = fetchedMessages,
    importedMessages = importedMessages,
    droppedUntrustedMessages = droppedUntrustedMessages,
    ackedMessages = ackedMessages,
)

internal fun RelayPushStatsDto.toRelayPushStats(): RelayPushStats = RelayPushStats(
    trustedPeers = trustedPeers,
    postedMessages = postedMessages,
    fullSyncMessages = fullSyncMessages,
    incrementalSyncMessages = incrementalSyncMessages,
)
