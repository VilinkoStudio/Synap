package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.model.DiscoveredSyncPeer
import com.synap.app.data.model.LocalIdentity
import com.synap.app.data.model.PeerRecord
import com.synap.app.data.model.PeerTrustStatus
import com.synap.app.data.model.SyncConnectionRecord
import com.synap.app.data.model.SyncListenerState
import com.synap.app.data.model.SyncSessionRecord
import com.synap.app.data.model.SyncStatus
import com.synap.app.data.repository.SyncRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class SyncUiState(
    val isLoading: Boolean = true,
    val isManagingPeer: Boolean = false,
    val isPairing: Boolean = false,
    val listenerState: SyncListenerState = SyncListenerState(),
    val localIdentity: LocalIdentity? = null,
    val discoveredPeers: List<DiscoveredSyncPeer> = emptyList(),
    val connections: List<SyncConnectionRecord> = emptyList(),
    val peers: List<PeerRecord> = emptyList(),
    val pendingTrustPeer: PeerRecord? = null,
    val recentSyncSessions: List<SyncSessionRecord> = emptyList(),
    val errorMessage: String? = null,
)

@HiltViewModel
class SyncViewModel @Inject constructor(
    private val repository: SyncRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(SyncUiState())
    val uiState: StateFlow<SyncUiState> = _uiState.asStateFlow()

    init {
        viewModelScope.launch {
            repository.runtimeState.collect { state ->
                _uiState.update { it.copy(listenerState = state) }
            }
        }
        viewModelScope.launch {
            repository.connections.collect { connections ->
                _uiState.update { it.copy(connections = connections) }
            }
        }
        viewModelScope.launch {
            repository.discoveredPeers.collect { discoveredPeers ->
                _uiState.update { it.copy(discoveredPeers = discoveredPeers) }
            }
        }
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            val listenerError = runCatching {
                repository.ensureListenerStarted()
            }.exceptionOrNull()

            val identityResult = runCatching { repository.getLocalIdentity() }
            val peersResult = runCatching { repository.getPeers() }
            val syncSessionsResult = runCatching { repository.getRecentSyncSessions(10u) }

            val errorMessages = buildList {
                listenerError?.message?.takeIf(String::isNotBlank)?.let { add("监听失败: $it") }
                identityResult.exceptionOrNull()?.message?.takeIf(String::isNotBlank)?.let {
                    add("读取本机身份失败: $it")
                }
                peersResult.exceptionOrNull()?.message?.takeIf(String::isNotBlank)?.let {
                    add("读取设备列表失败: $it")
                }
                syncSessionsResult.exceptionOrNull()?.message?.takeIf(String::isNotBlank)?.let {
                    add("读取同步统计失败: $it")
                }
            }

            _uiState.update { current ->
                current.copy(
                    isLoading = false,
                    localIdentity = identityResult.getOrNull() ?: current.localIdentity,
                    peers = peersResult.getOrNull() ?: current.peers,
                    recentSyncSessions = syncSessionsResult.getOrNull() ?: current.recentSyncSessions,
                    errorMessage = errorMessages.takeIf { it.isNotEmpty() }?.joinToString("\n"),
                )
            }
        }
    }

    fun trustPeer(publicKey: ByteArray, note: String?) {
        viewModelScope.launch {
            _uiState.update { it.copy(isManagingPeer = true, errorMessage = null) }
            runCatching {
                val trustedPeer = repository.trustPeer(publicKey, note?.takeIf(String::isNotBlank))
                val peers = repository.getPeers()
                trustedPeer to peers
            }.fold(
                onSuccess = { (trustedPeer, peers) ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            peers = peers,
                            pendingTrustPeer = it.pendingTrustPeer?.takeUnless { pending ->
                                pending.id == trustedPeer.id
                            },
                            errorMessage = null,
                        )
                    }
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            errorMessage = throwable.message ?: "信任对端失败",
                        )
                    }
                },
            )
        }
    }

    fun addConnection(host: String, port: Int) {
        viewModelScope.launch {
            _uiState.update { it.copy(errorMessage = null) }
            runCatching {
                repository.addConnection(host, port)
            }.onFailure { throwable ->
                _uiState.update {
                    it.copy(errorMessage = throwable.message ?: "保存连接失败")
                }
            }
        }
    }

    fun deleteConnection(connectionId: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(errorMessage = null) }
            runCatching {
                repository.deleteConnection(connectionId)
            }.onFailure { throwable ->
                _uiState.update {
                    it.copy(errorMessage = throwable.message ?: "删除连接失败")
                }
            }
        }
    }

    fun pairConnection(connectionId: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(isPairing = true, errorMessage = null) }
            runCatching {
                val session = repository.pairConnection(connectionId)
                val peers = repository.getPeers()
                val syncSessions = repository.getRecentSyncSessions(10u)
                Triple(session, peers, syncSessions)
            }.fold(
                onSuccess = { (session, peers, syncSessions) ->
                    _uiState.update {
                        it.copy(
                            isPairing = false,
                            peers = peers,
                            pendingTrustPeer = session.peer.takeIf {
                                session.status == SyncStatus.PendingTrust
                            },
                            recentSyncSessions = syncSessions,
                            errorMessage = null,
                        )
                    }
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isPairing = false,
                            errorMessage = throwable.message ?: "配对失败",
                        )
                    }
                },
            )
        }
    }

    fun pairDiscoveredPeer(host: String, port: Int) {
        viewModelScope.launch {
            _uiState.update { it.copy(isPairing = true, errorMessage = null) }
            runCatching {
                val session = repository.pairEndpoint(host, port)
                val peers = repository.getPeers()
                val syncSessions = repository.getRecentSyncSessions(10u)
                Triple(session, peers, syncSessions)
            }.fold(
                onSuccess = { (session, peers, syncSessions) ->
                    _uiState.update {
                        it.copy(
                            isPairing = false,
                            peers = peers,
                            pendingTrustPeer = session.peer.takeIf {
                                session.status == SyncStatus.PendingTrust
                            },
                            recentSyncSessions = syncSessions,
                            errorMessage = null,
                        )
                    }
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isPairing = false,
                            errorMessage = throwable.message ?: "配对失败",
                        )
                    }
                },
            )
        }
    }

    fun updatePeerNote(peerId: String, note: String?) {
        viewModelScope.launch {
            _uiState.update { it.copy(isManagingPeer = true, errorMessage = null) }
            runCatching {
                repository.updatePeerNote(peerId, note?.takeIf(String::isNotBlank))
                repository.getPeers()
            }.fold(
                onSuccess = { peers ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            peers = peers,
                            pendingTrustPeer = it.pendingTrustPeer?.let { pending ->
                                peers.firstOrNull { peer -> peer.id == pending.id }
                            },
                            errorMessage = null,
                        )
                    }
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            errorMessage = throwable.message ?: "更新设备备注失败",
                        )
                    }
                },
            )
        }
    }

    fun setPeerStatus(peerId: String, status: PeerTrustStatus) {
        viewModelScope.launch {
            _uiState.update { it.copy(isManagingPeer = true, errorMessage = null) }
            runCatching {
                val updatedPeer = repository.setPeerStatus(peerId, status)
                val peers = repository.getPeers()
                updatedPeer to peers
            }.fold(
                onSuccess = { (updatedPeer, peers) ->
                    _uiState.update {
                        val pendingTrustPeer = when {
                            it.pendingTrustPeer?.id != updatedPeer.id -> {
                                it.pendingTrustPeer?.let { pending ->
                                    peers.firstOrNull { peer -> peer.id == pending.id }
                                }
                            }
                            updatedPeer.status == PeerTrustStatus.Pending -> updatedPeer
                            else -> null
                        }
                        it.copy(
                            isManagingPeer = false,
                            peers = peers,
                            pendingTrustPeer = pendingTrustPeer,
                            errorMessage = null,
                        )
                    }
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            errorMessage = throwable.message ?: "更新设备状态失败",
                        )
                    }
                },
            )
        }
    }

    fun deletePeer(peerId: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(isManagingPeer = true, errorMessage = null) }
            runCatching {
                repository.deletePeer(peerId)
                repository.getPeers()
            }.fold(
                onSuccess = { peers ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            peers = peers,
                            pendingTrustPeer = it.pendingTrustPeer?.let { pending ->
                                peers.firstOrNull { peer -> peer.id == pending.id }
                            },
                            errorMessage = null,
                        )
                    }
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isManagingPeer = false,
                            errorMessage = throwable.message ?: "删除设备失败",
                        )
                    }
                },
            )
        }
    }

    fun dismissPendingTrustPrompt() {
        _uiState.update { it.copy(pendingTrustPeer = null) }
    }
}
