package com.synap.app.data.repository

import android.util.Log
import com.synap.app.data.model.DiscoveredSyncPeer
import com.synap.app.data.model.LocalIdentity
import com.synap.app.data.model.PeerRecord
import com.synap.app.data.model.PeerTrustStatus
import com.synap.app.data.model.ShareImportStats
import com.synap.app.data.model.SyncConnectionRecord
import com.synap.app.data.model.SyncConnectionStatus
import com.synap.app.data.model.SyncListenerState
import com.synap.app.data.model.SyncSession
import com.synap.app.data.model.SyncSessionRecord
import com.synap.app.data.model.SyncStatus
import com.synap.app.data.service.SyncConnectConfig
import com.synap.app.data.service.SyncConnectionStore
import com.synap.app.data.service.SyncDiscoveryRuntime
import com.synap.app.data.service.SyncListenConfig
import com.synap.app.data.service.SynapServiceApi
import com.synap.app.data.service.SyncNetworkRuntime
import com.synap.app.di.IoDispatcher
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch

private const val TAG = "SyncRepository"
private const val DEFAULT_SYNC_PORT = 45_172

interface SyncRepository {
    val runtimeState: StateFlow<SyncListenerState>
    val connections: StateFlow<List<SyncConnectionRecord>>
    val discoveredPeers: StateFlow<List<DiscoveredSyncPeer>>

    suspend fun ensureListenerStarted()

    suspend fun addConnection(host: String, port: Int): SyncConnectionRecord

    suspend fun deleteConnection(connectionId: String)

    suspend fun pairConnection(connectionId: String): SyncSession

    suspend fun pairEndpoint(host: String, port: Int): SyncSession

    suspend fun getLocalIdentity(): LocalIdentity

    suspend fun getPeers(): List<PeerRecord>

    suspend fun trustPeer(publicKey: ByteArray, note: String?): PeerRecord

    suspend fun updatePeerNote(peerId: String, note: String?): PeerRecord

    suspend fun setPeerStatus(peerId: String, status: PeerTrustStatus): PeerRecord

    suspend fun deletePeer(peerId: String)

    suspend fun getRecentSyncSessions(limit: UInt? = null): List<SyncSessionRecord>
}

@Singleton
class SyncRepositoryImpl @Inject constructor(
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
    private val service: SynapServiceApi,
    private val runtime: SyncNetworkRuntime,
    private val discoveryRuntime: SyncDiscoveryRuntime,
    private val connectionStore: SyncConnectionStore,
    private val mutationStore: SynapMutationStore,
) : SyncRepository {
    private val scope = CoroutineScope(SupervisorJob() + ioDispatcher)
    override val runtimeState: StateFlow<SyncListenerState> = runtime.state
    private val _connections = MutableStateFlow(connectionStore.list())
    override val connections: StateFlow<List<SyncConnectionRecord>> = _connections.asStateFlow()
    override val discoveredPeers: StateFlow<List<DiscoveredSyncPeer>> = discoveryRuntime.discoveredPeers

    init {
        scope.launch {
            runtime.incomingChannels.collect { channel ->
                scope.launch {
                    runCatching {
                        try {
                            service.listenSync(channel).unwrap()
                        } finally {
                            channel.close()
                        }
                    }.onSuccess { session ->
                        Log.i(TAG, "Inbound sync completed with status=${session.status}")
                        emitImportedMutationIfNeeded(session)
                    }.onFailure { throwable ->
                        Log.e(TAG, "Inbound sync failed", throwable)
                    }
                }
            }
        }
    }

    override suspend fun ensureListenerStarted() {
        val listenerState = runCatching {
            runtime.ensureStarted(SyncListenConfig(port = DEFAULT_SYNC_PORT)).unwrap()
        }.getOrElse {
            runtime.ensureStarted().unwrap()
        }
        listenerState.listenPort?.let { port ->
            discoveryRuntime.ensureStarted(port).unwrap()
        }
    }

    override suspend fun addConnection(host: String, port: Int): SyncConnectionRecord {
        val normalizedHost = host.trim()
        require(normalizedHost.isNotEmpty()) { "主机地址不能为空" }
        require(port in 1..65535) { "端口必须在 1 到 65535 之间" }

        val record = connectionStore.create(
            name = "$normalizedHost:$port",
            host = normalizedHost,
            port = port,
        )
        _connections.value = connectionStore.list()
        return record
    }

    override suspend fun deleteConnection(connectionId: String) {
        connectionStore.delete(connectionId)
        _connections.value = connectionStore.list()
    }

    override suspend fun pairConnection(connectionId: String): SyncSession {
        val existing = _connections.value.firstOrNull { it.id == connectionId }
            ?: error("找不到连接记录")
        updateConnection(
            existing.copy(
                status = SyncConnectionStatus.Connecting,
                statusMessage = "正在建立 TCP 通道...",
            ),
        )

        return runCatching { initiateSync(existing.host, existing.port) }.fold(
            onSuccess = { session ->
                updateConnection(
                    existing.copy(
                        status = when (session.status) {
                            SyncStatus.Completed -> SyncConnectionStatus.Connected
                            SyncStatus.PendingTrust -> SyncConnectionStatus.AwaitingTrust
                            SyncStatus.Failed -> SyncConnectionStatus.Failed
                        },
                        statusMessage = when (session.status) {
                            SyncStatus.Completed -> "同步完成"
                            SyncStatus.PendingTrust -> "发现陌生设备，等待信任确认"
                            SyncStatus.Failed -> "同步失败"
                        },
                    ),
                )
                session
            },
            onFailure = { throwable ->
                updateConnection(
                    existing.copy(
                        status = SyncConnectionStatus.Failed,
                        statusMessage = throwable.message ?: "配对失败",
                    ),
                )
                throw throwable
            },
        )
    }

    override suspend fun pairEndpoint(host: String, port: Int): SyncSession =
        initiateSync(host.trim(), port)

    override suspend fun getLocalIdentity(): LocalIdentity =
        service.getLocalIdentity().unwrap()

    override suspend fun getPeers(): List<PeerRecord> =
        service.getPeers().unwrap()

    override suspend fun trustPeer(publicKey: ByteArray, note: String?): PeerRecord =
        service.trustPeer(publicKey, note).unwrap()

    override suspend fun updatePeerNote(peerId: String, note: String?): PeerRecord =
        service.updatePeerNote(peerId, note).unwrap()

    override suspend fun setPeerStatus(peerId: String, status: PeerTrustStatus): PeerRecord =
        service.setPeerStatus(peerId, status).unwrap()

    override suspend fun deletePeer(peerId: String) {
        service.deletePeer(peerId).unwrap()
    }

    override suspend fun getRecentSyncSessions(limit: UInt?): List<SyncSessionRecord> =
        service.getRecentSyncSessions(limit).unwrap()

    private fun updateConnection(record: SyncConnectionRecord) {
        connectionStore.update(record)
        _connections.value = connectionStore.list()
    }

    private suspend fun initiateSync(host: String, port: Int) = run {
        val channel = runtime.connect(
            SyncConnectConfig(
                host = host,
                port = port,
            ),
        ).unwrap()
        try {
            service.initiateSync(channel).unwrap().also(::emitImportedMutationIfNeeded)
        } finally {
            channel.close()
        }
    }

    private fun emitImportedMutationIfNeeded(session: SyncSession) {
        if (session.status != SyncStatus.Completed) {
            return
        }
        val stats = session.stats ?: return
        mutationStore.emit(
            SynapMutation.Imported(
                ShareImportStats(
                    records = stats.recordsReceived.toLong(),
                    recordsApplied = stats.recordsApplied.toLong(),
                    bytes = stats.bytesReceived.toLong(),
                    durationMs = stats.durationMs.toLong(),
                ),
            ),
        )
    }

    private fun <T> Result<T>.unwrap(): T = getOrElse { throw it }
}
