package com.synap.app.ui.screens

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.synap.app.data.model.LocalIdentity
import com.synap.app.data.model.SyncConnectionRecord
import com.synap.app.data.model.SyncConnectionStatus
import com.synap.app.data.model.DiscoveredSyncPeer
import com.synap.app.data.model.PeerRecord
import com.synap.app.data.model.PeerTrustStatus
import com.synap.app.data.model.SyncSessionRecord
import com.synap.app.data.model.SyncSessionRole
import com.synap.app.data.model.SyncStatus
import com.synap.app.ui.viewmodel.SyncUiState
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.concurrent.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SyncScreen(
    uiState: SyncUiState,
    onRefresh: () -> Unit,
    onAddConnection: (String, Int) -> Unit,
    onDeleteConnection: (String) -> Unit,
    onPairConnection: (String) -> Unit,
    onPairDiscoveredPeer: (String, Int) -> Unit,
    onRelayBaseUrlChange: (String) -> Unit,
    onRelayApiKeyChange: (String) -> Unit,
    onSaveRelayConfig: () -> Unit,
    onRelayFetch: () -> Unit,
    onRelayPush: () -> Unit,
    onTrustPeer: (ByteArray, String?) -> Unit,
    onUpdatePeerNote: (String, String?) -> Unit,
    onDeletePeer: (String) -> Unit,
    onSetPeerStatus: (String, PeerTrustStatus) -> Unit,
    onDismissPendingTrustPrompt: () -> Unit,
    onNavigateBack: () -> Unit,
) {
    var backProgress by remember { mutableFloatStateOf(0f) }
    var selectedSyncSession by remember { mutableStateOf<SyncSessionRecord?>(null) }
    var showAddConnectionDialog by remember { mutableStateOf(false) }
    var connectionHost by remember { mutableStateOf("") }
    var connectionPort by remember { mutableStateOf("") }

    // ========== 新增：控制监听状态弹窗的开关 ==========
    var showListeningInfoDialog by remember { mutableStateOf(false) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress
            }
            onNavigateBack()
        } catch (_: CancellationException) {
            backProgress = 0f
        }
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            .graphicsLayer {
                translationX = backProgress * 64.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            },
        topBar = {
            TopAppBar(
                title = {
                    // ========== 核心修改 1：在标题旁增加监听状态文字按钮 ==========
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("同步")
                        Spacer(modifier = Modifier.width(12.dp))
                        Button(
                            onClick = { showListeningInfoDialog = true },
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
                        ) {
                            Text(
                                text = if (uiState.listenerState.isListening) "正在监听" else "未监听",
                                style = MaterialTheme.typography.labelSmall
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
                actions = {
                    IconButton(onClick = onRefresh) {
                        Icon(Icons.Filled.Refresh, contentDescription = "刷新")
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Spacer(modifier = Modifier.height(4.dp))

            // 原本的 SyncOverviewCard 已经被移除，内容已转移至下方的 showListeningInfoDialog 中

            uiState.errorMessage?.let { message ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                    ),
                ) {
                    Text(
                        text = message,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        modifier = Modifier.padding(16.dp),
                    )
                }
            }

            IdentitySection(identity = uiState.localIdentity)

            ConnectionSection(
                discoveredPeers = uiState.discoveredPeers,
                connections = uiState.connections,
                isPairing = uiState.isPairing,
                onAddConnection = { showAddConnectionDialog = true },
                onDeleteConnection = onDeleteConnection,
                onPairConnection = onPairConnection,
                onPairDiscoveredPeer = onPairDiscoveredPeer,
            )

            RelaySection(
                baseUrl = uiState.relayBaseUrl,
                apiKey = uiState.relayApiKey,
                statusMessage = uiState.relayStatusMessage,
                isSyncing = uiState.isRelaySyncing,
                onBaseUrlChange = onRelayBaseUrlChange,
                onApiKeyChange = onRelayApiKeyChange,
                onSave = onSaveRelayConfig,
                onFetch = onRelayFetch,
                onPush = onRelayPush,
            )

            PeerSection(
                peers = uiState.peers,
                isManagingPeer = uiState.isManagingPeer,
                pendingTrustPeerId = uiState.pendingTrustPeer?.id,
                onTrustPeer = onTrustPeer,
                onUpdatePeerNote = onUpdatePeerNote,
                onDeletePeer = onDeletePeer,
                onSetPeerStatus = onSetPeerStatus,
                onDismissPendingTrustPrompt = onDismissPendingTrustPrompt,
            )

            SyncStatsSection(
                sessions = uiState.recentSyncSessions,
                onOpenSession = { session -> selectedSyncSession = session },
            )

            Spacer(modifier = Modifier.height(24.dp))
        }
    }

    // ========== 核心修改 1 延伸：监听信息的弹窗 ==========
    if (showListeningInfoDialog) {
        AlertDialog(
            onDismissRequest = { showListeningInfoDialog = false },
            icon = { Icon(Icons.Filled.Wifi, contentDescription = null) },
            title = { Text("监听状态") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    SyncInfoLine("协议", uiState.listenerState.protocol)
                    SyncInfoLine("实现", uiState.listenerState.backend)
                    SyncInfoLine("监听端口", uiState.listenerState.listenPort?.toString() ?: "未分配")
                    SyncInfoLine(
                        "局域网地址",
                        uiState.listenerState.localAddresses.takeIf { it.isNotEmpty() }?.joinToString(", ") ?: "未获取到局域网地址",
                    )
                    SyncInfoLine("状态", if (uiState.listenerState.isListening) "正在监听" else "未监听")
                }
            },
            confirmButton = {
                TextButton(onClick = { showListeningInfoDialog = false }) {
                    Text("关闭")
                }
            }
        )
    }

    if (showAddConnectionDialog) {
        AlertDialog(
            onDismissRequest = { showAddConnectionDialog = false },
            icon = { Icon(Icons.Filled.Add, contentDescription = null) },
            title = { Text("添加连接") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = connectionHost,
                        onValueChange = { connectionHost = it },
                        label = { Text("主机地址") },
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = connectionPort,
                        onValueChange = { connectionPort = it.filter(Char::isDigit) },
                        label = { Text("端口") },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val port = connectionPort.toIntOrNull()
                        if (!connectionHost.isBlank() && port != null) {
                            onAddConnection(connectionHost.trim(), port)
                            showAddConnectionDialog = false
                            connectionHost = ""
                            connectionPort = ""
                        }
                    },
                    enabled = connectionHost.isNotBlank() && connectionPort.toIntOrNull() != null,
                ) {
                    Text("保存")
                }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        showAddConnectionDialog = false
                        connectionHost = ""
                        connectionPort = ""
                    },
                ) {
                    Text("取消")
                }
            },
        )
    }

    selectedSyncSession?.let { session ->
        AlertDialog(
            onDismissRequest = { selectedSyncSession = null },
            icon = { Icon(Icons.Filled.Sync, contentDescription = null) },
            title = { Text("同步详情") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    SyncInfoLine("结果", session.status.displayLabel())
                    SyncInfoLine("方向", session.role.displayLabel())
                    SyncInfoLine("设备", session.peerLabel ?: "未命名设备")
                    SyncInfoLine("完成时间", formatTimestamp(session.finishedAtMs.toLong()))
                    SyncInfoLine("开始时间", formatTimestamp(session.startedAtMs.toLong()))
                    SyncInfoLine("发送记录", session.recordsSent.toString())
                    SyncInfoLine("接收记录", session.recordsReceived.toString())
                    SyncInfoLine("应用记录", session.recordsApplied.toString())
                    SyncInfoLine("跳过记录", session.recordsSkipped.toString())
                    SyncInfoLine("发送字节", session.bytesSent.toString())
                    SyncInfoLine("接收字节", session.bytesReceived.toString())
                    SyncInfoLine("耗时", "${session.durationMs} ms")
                    session.peerFingerprint.takeIf { it.isNotEmpty() }?.let { _ ->
                        SyncDetailBlock("对端指纹", session.displayPeerFingerprintBase64)
                    }
                    session.errorMessage?.takeIf(String::isNotBlank)?.let { errorMessage ->
                        SyncDetailBlock("错误信息", errorMessage)
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { selectedSyncSession = null }) {
                    Text("关闭")
                }
            },
        )
    }
}

@Composable
private fun IdentitySection(identity: LocalIdentity?) {
    SectionTitle(title = "本机签名密钥", subtitle = "Ed25519 签名密钥，用于设备身份认证")

    if (identity == null) {
        EmptySectionCard("正在读取此设备信息")
        return
    }

    val signingAvatar = remember(identity.signing.avatarPng) {
        BitmapFactory.decodeByteArray(identity.signing.avatarPng, 0, identity.signing.avatarPng.size)
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            PublicKeyAvatar(
                bitmap = signingAvatar,
                contentDescription = "签名密钥头像",
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(14.dp)),
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = "签名密钥",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Text(
                    text = identity.signing.displayPublicKeyBase64,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text(
                text = "Ed25519",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .background(
                        color = MaterialTheme.colorScheme.surfaceContainerHighest,
                        shape = RoundedCornerShape(8.dp),
                    )
                    .padding(horizontal = 10.dp, vertical = 4.dp),
            )
        }
    }
}

@Composable
private fun ConnectionSection(
    discoveredPeers: List<DiscoveredSyncPeer>,
    connections: List<SyncConnectionRecord>,
    isPairing: Boolean,
    onAddConnection: () -> Unit,
    onDeleteConnection: (String) -> Unit,
    onPairConnection: (String) -> Unit,
    onPairDiscoveredPeer: (String, Int) -> Unit,
) {
    SectionTitle(title = "可用连接", subtitle = "局域网发现和手动添加的地址都可以直接发起同步")

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Column {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "连接目标",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = "发现到的设备可直接配对，也可以手动添加地址端口",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                FilledTonalButton(onClick = onAddConnection) {
                    Icon(
                        imageVector = Icons.Filled.Add,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("添加连接")
                }
            }

            if (discoveredPeers.isEmpty() && connections.isEmpty()) {
                EmptySectionCard("暂无可用连接，请确保和其他设备处于同一局域网下，或者您可以尝试手动添加地址和端口。")
            } else {
                discoveredPeers.forEachIndexed { index, peer ->
                    if (index > 0) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                            modifier = Modifier.padding(horizontal = 16.dp),
                        )
                    }
                    DiscoveredConnectionRow(
                        peer = peer,
                        isPairing = isPairing,
                        onPairDiscoveredPeer = onPairDiscoveredPeer,
                    )
                }

                if (discoveredPeers.isNotEmpty() && connections.isNotEmpty()) {
                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )
                }

                connections.forEachIndexed { index, connection ->
                    if (index > 0 || discoveredPeers.isNotEmpty()) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                            modifier = Modifier.padding(horizontal = 16.dp),
                        )
                    }
                    ConnectionRow(
                        connection = connection,
                        isPairing = isPairing,
                        onDeleteConnection = onDeleteConnection,
                        onPairConnection = onPairConnection,
                    )
                }
            }
        }
    }
}

@Composable
private fun RelaySection(
    baseUrl: String,
    apiKey: String,
    statusMessage: String?,
    isSyncing: Boolean,
    onBaseUrlChange: (String) -> Unit,
    onApiKeyChange: (String) -> Unit,
    onSave: () -> Unit,
    onFetch: () -> Unit,
    onPush: () -> Unit,
) {
    SectionTitle(title = "Relay Server", subtitle = "跨网络同步使用的零信任中继，只保存地址和 API Key")

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            OutlinedTextField(
                value = baseUrl,
                onValueChange = onBaseUrlChange,
                label = { Text("Relay 地址") },
                placeholder = { Text("http://relay.example.com:8080") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )
            OutlinedTextField(
                value = apiKey,
                onValueChange = onApiKeyChange,
                label = { Text("API Key") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )
            statusMessage?.takeIf(String::isNotBlank)?.let { message ->
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                FilledTonalButton(
                    onClick = onSave,
                    enabled = !isSyncing,
                    modifier = Modifier.weight(1f),
                ) {
                    Text("保存")
                }
                FilledTonalButton(
                    onClick = onFetch,
                    enabled = !isSyncing && baseUrl.isNotBlank(),
                    modifier = Modifier.weight(1f),
                ) {
                    Text("拉取")
                }
                Button(
                    onClick = onPush,
                    enabled = !isSyncing && baseUrl.isNotBlank(),
                    modifier = Modifier.weight(1f),
                ) {
                    Text("推送")
                }
            }
        }
    }
}

@Composable
private fun DiscoveredConnectionRow(
    peer: DiscoveredSyncPeer,
    isPairing: Boolean,
    onPairDiscoveredPeer: (String, Int) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = Icons.Filled.Sync,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = peer.displayName,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "${peer.host}:${peer.port}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = "局域网发现",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
            )
        }
        FilledTonalButton(
            onClick = { onPairDiscoveredPeer(peer.host, peer.port) },
            enabled = !isPairing,
        ) {
            Text("配对")
        }
    }
}

@Composable
private fun PeerSection(
    peers: List<PeerRecord>,
    isManagingPeer: Boolean,
    pendingTrustPeerId: String?,
    onTrustPeer: (ByteArray, String?) -> Unit,
    onUpdatePeerNote: (String, String?) -> Unit,
    onDeletePeer: (String) -> Unit,
    onSetPeerStatus: (String, PeerTrustStatus) -> Unit,
    onDismissPendingTrustPrompt: () -> Unit,
) {
    SectionTitle(title = "设备列表", subtitle = "包含已信任、待确认和已撤销的对端公钥")

    if (peers.isEmpty()) {
        EmptySectionCard("还没有发现任何局域网内的设备")
        return
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Column {
            peers.forEachIndexed { index, peer ->
                if (index > 0) {
                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )
                }
                PeerRow(
                    peer = peer,
                    isManagingPeer = isManagingPeer,
                    isPendingTrust = peer.id == pendingTrustPeerId,
                    onTrustPeer = { onTrustPeer(peer.publicKey, it) },
                    onUpdatePeerNote = { note -> onUpdatePeerNote(peer.id, note) },
                    onDeletePeer = { onDeletePeer(peer.id) },
                    onSetPeerStatus = { status -> onSetPeerStatus(peer.id, status) },
                    onDismissPendingTrustPrompt = onDismissPendingTrustPrompt,
                )
            }
        }
    }
}

@Composable
private fun PeerRow(
    peer: PeerRecord,
    isManagingPeer: Boolean,
    isPendingTrust: Boolean,
    onTrustPeer: (String?) -> Unit,
    onUpdatePeerNote: (String?) -> Unit,
    onDeletePeer: () -> Unit,
    onSetPeerStatus: (PeerTrustStatus) -> Unit,
    onDismissPendingTrustPrompt: () -> Unit,
) {
    var expanded by remember { mutableStateOf(peer.status == PeerTrustStatus.Pending) }
    var noteDraft by remember { mutableStateOf(peer.note.orEmpty()) }

    val avatarBitmap = remember(peer.avatarPng) {
        BitmapFactory.decodeByteArray(peer.avatarPng, 0, peer.avatarPng.size)
    }

    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clickable { expanded = !expanded }
                .padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            PublicKeyAvatar(
                bitmap = avatarBitmap,
                contentDescription = "设备头像",
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(14.dp)),
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = peer.note ?: "未命名设备",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Text(
                    text = peer.displayPublicKeyBase64,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            StatusChip(status = peer.status)
            Spacer(modifier = Modifier.width(4.dp))
            Icon(
                imageVector = if (expanded) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
                contentDescription = if (expanded) "收起" else "展开",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        AnimatedVisibility(
            visible = expanded,
            enter = expandVertically(),
            exit = shrinkVertically(),
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 16.dp, end = 16.dp, bottom = 14.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    OutlinedTextField(
                        value = noteDraft,
                        onValueChange = { noteDraft = it },
                        label = { Text("备注") },
                        modifier = Modifier.weight(1f),
                        singleLine = true,
                    )
                    FilledTonalButton(
                        onClick = { onUpdatePeerNote(noteDraft.ifBlank { null }) },
                        enabled = !isManagingPeer,
                    ) {
                        Text("保存")
                    }
                }

                if (peer.status == PeerTrustStatus.Pending) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Button(
                            onClick = {
                                onTrustPeer(noteDraft.ifBlank { null })
                                onDismissPendingTrustPrompt()
                            },
                            enabled = !isManagingPeer,
                            modifier = Modifier.weight(1f),
                        ) {
                            Text("信任")
                        }
                        FilledTonalButton(
                            onClick = {
                                onSetPeerStatus(PeerTrustStatus.Revoked)
                                onDismissPendingTrustPrompt()
                            },
                            enabled = !isManagingPeer,
                            modifier = Modifier.weight(1f),
                        ) {
                            Text("拒绝")
                        }
                    }
                } else {
                    val targetStatus = if (peer.status == PeerTrustStatus.Trusted) {
                        PeerTrustStatus.Revoked
                    } else {
                        PeerTrustStatus.Trusted
                    }
                    val actionLabel = if (peer.status == PeerTrustStatus.Trusted) "撤销信任" else "设为可信"
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        FilledTonalButton(
                            onClick = { onSetPeerStatus(targetStatus) },
                            enabled = !isManagingPeer,
                            modifier = Modifier.weight(1f),
                        ) {
                            Text(actionLabel)
                        }
                        TextButton(
                            onClick = onDeletePeer,
                            enabled = !isManagingPeer,
                            modifier = Modifier.weight(1f),
                        ) {
                            Text("删除", color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun StatusChip(status: PeerTrustStatus) {
    val (label, containerColor, contentColor) = when (status) {
        PeerTrustStatus.Pending -> Triple(
            "待确认",
            MaterialTheme.colorScheme.primaryContainer,
            MaterialTheme.colorScheme.onPrimaryContainer,
        )
        PeerTrustStatus.Trusted -> Triple(
            "已信任",
            MaterialTheme.colorScheme.tertiaryContainer,
            MaterialTheme.colorScheme.onTertiaryContainer,
        )
        PeerTrustStatus.Retired -> Triple(
            "已停用",
            MaterialTheme.colorScheme.secondaryContainer,
            MaterialTheme.colorScheme.onSecondaryContainer,
        )
        PeerTrustStatus.Revoked -> Triple(
            "已撤销",
            MaterialTheme.colorScheme.errorContainer,
            MaterialTheme.colorScheme.onErrorContainer,
        )
    }
    Text(
        text = label,
        style = MaterialTheme.typography.labelSmall,
        color = contentColor,
        modifier = Modifier
            .background(containerColor, RoundedCornerShape(6.dp))
            .padding(horizontal = 8.dp, vertical = 4.dp),
    )
}

@Composable
private fun SyncStatsSection(
    sessions: List<SyncSessionRecord>,
    onOpenSession: (SyncSessionRecord) -> Unit,
) {
    SectionTitle(title = "同步统计", subtitle = "展示最近一次和最近几次同步结果")

    val latest = sessions.firstOrNull()
    if (latest == null) {
        EmptySectionCard("还没有任何同步记录")
        return
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                text = "最近一次同步",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface,
            )
            SyncInfoLine("时间", formatTimestamp(latest.finishedAtMs.toLong()))
            SyncInfoLine("结果", latest.status.displayLabel())
            SyncInfoLine("方向", latest.role.displayLabel())
            SyncInfoLine("设备", latest.peerLabel ?: "未命名设备")
            SyncInfoLine("应用记录", latest.recordsApplied.toString())
            SyncInfoLine("发送/接收", "${latest.recordsSent}/${latest.recordsReceived}")
            SyncInfoLine("耗时", "${latest.durationMs} ms")

            if (sessions.size > 1) {
                HorizontalDivider(
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                )
                Text(
                    text = "最近记录",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                sessions.take(5).forEachIndexed { index, session ->
                    if (index > 0) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        )
                    }
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onOpenSession(session) }
                            .padding(vertical = 2.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text(
                            text = "${session.role.displayLabel()} · ${session.status.displayLabel()}",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = "${formatTimestamp(session.finishedAtMs.toLong())} · ${session.peerLabel ?: "未命名设备"}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SyncDetailBlock(label: String, value: String) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}

@Composable
private fun SectionTitle(title: String, subtitle: String) {
    Column(
        modifier = Modifier.padding(start = 4.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.primary,
        )
        Text(
            text = subtitle,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun PublicKeyAvatar(
    bitmap: Bitmap?,
    contentDescription: String,
    modifier: Modifier = Modifier,
) {
    if (bitmap != null) {
        Image(
            bitmap = bitmap.asImageBitmap(),
            contentDescription = contentDescription,
            modifier = modifier,
        )
    } else {
        Box(
            modifier = modifier.background(
                color = MaterialTheme.colorScheme.secondaryContainer,
                shape = RoundedCornerShape(14.dp),
            ),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = Icons.Filled.Key,
                contentDescription = "$contentDescription 占位",
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(20.dp),
            )
        }
    }
}

@Composable
private fun ConnectionRow(
    connection: SyncConnectionRecord,
    isPairing: Boolean,
    onDeleteConnection: (String) -> Unit,
    onPairConnection: (String) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = Icons.Filled.Wifi,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = connection.name,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "${connection.host}:${connection.port}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = connection.statusMessage,
                style = MaterialTheme.typography.labelLarge,
                color = connection.status.color(),
            )
        }
        FilledTonalButton(
            onClick = { onPairConnection(connection.id) },
            enabled = !isPairing,
        ) {
            Text("配对")
        }
        IconButton(onClick = { onDeleteConnection(connection.id) }) {
            Icon(
                imageVector = Icons.Filled.DeleteOutline,
                contentDescription = "删除连接",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun EmptySectionCard(message: String) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Text(
            text = message,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(16.dp),
        )
    }
}

@Composable
private fun SyncInfoLine(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f)
        )
    }
}

@Composable
private fun SyncConnectionStatus.color() = when (this) {
    SyncConnectionStatus.Idle -> MaterialTheme.colorScheme.secondary
    SyncConnectionStatus.Connecting -> MaterialTheme.colorScheme.primary
    SyncConnectionStatus.AwaitingTrust -> MaterialTheme.colorScheme.tertiary
    SyncConnectionStatus.Connected -> MaterialTheme.colorScheme.tertiary
    SyncConnectionStatus.Failed -> MaterialTheme.colorScheme.error
}

private fun SyncStatus.displayLabel(): String = when (this) {
    SyncStatus.Completed -> "已完成"
    SyncStatus.PendingTrust -> "待信任"
    SyncStatus.Failed -> "失败"
}

private fun SyncSessionRole.displayLabel(): String = when (this) {
    SyncSessionRole.Initiator -> "主动发起"
    SyncSessionRole.Listener -> "被动接收"
    SyncSessionRole.RelayFetch -> "Relay 拉取"
    SyncSessionRole.RelayPush -> "Relay 推送"
}

private fun formatTimestamp(timestampMs: Long): String =
    DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")
        .format(Instant.ofEpochMilli(timestampMs).atZone(ZoneId.systemDefault()))
