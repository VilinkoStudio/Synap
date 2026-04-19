package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.Devices
import androidx.compose.material.icons.filled.Key
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
import androidx.compose.material3.SuggestionChip
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.font.FontFamily
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
    onTrustPeer: (ByteArray, String?) -> Unit,
    onUpdatePeerNote: (String, String?) -> Unit,
    onDeletePeer: (String) -> Unit,
    onSetPeerStatus: (String, PeerTrustStatus) -> Unit,
    onDismissPendingTrustPrompt: () -> Unit,
    onNavigateBack: () -> Unit,
) {
    var backProgress by remember { mutableFloatStateOf(0f) }
    var activePeer by remember { mutableStateOf<PeerRecord?>(null) }
    var selectedSyncSession by remember { mutableStateOf<SyncSessionRecord?>(null) }
    var peerNoteDraft by remember { mutableStateOf("") }
    var showAddConnectionDialog by remember { mutableStateOf(false) }
    var connectionHost by remember { mutableStateOf("") }
    var connectionPort by remember { mutableStateOf("") }

    LaunchedEffect(uiState.pendingTrustPeer?.id) {
        uiState.pendingTrustPeer?.let { peer ->
            activePeer = peer
            peerNoteDraft = peer.note.orEmpty()
        }
    }

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
                val scale = 1f - (0.1f * backProgress)
                scaleX = scale
                scaleY = scale
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            },
        topBar = {
            TopAppBar(
                title = { Text("同步") },
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

            SyncOverviewCard(
                status = uiState.listenerState.status,
                protocol = uiState.listenerState.protocol,
                backend = uiState.listenerState.backend,
                port = uiState.listenerState.listenPort,
                localAddresses = uiState.listenerState.localAddresses,
                isListening = uiState.listenerState.isListening,
                onRefresh = onRefresh,
            )

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

            PeerSection(
                peers = uiState.peers,
                isManagingPeer = uiState.isManagingPeer,
                onManagePeer = { peer ->
                    activePeer = peer
                    peerNoteDraft = peer.note.orEmpty()
                },
            )

            SyncStatsSection(
                sessions = uiState.recentSyncSessions,
                onOpenSession = { session -> selectedSyncSession = session },
            )

            Spacer(modifier = Modifier.height(24.dp))
        }
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

    activePeer?.let { peer ->
        AlertDialog(
            onDismissRequest = {
                if (uiState.pendingTrustPeer?.id == peer.id) {
                    onDismissPendingTrustPrompt()
                }
                activePeer = null
                peerNoteDraft = ""
            },
            icon = { Icon(Icons.Filled.Key, contentDescription = null) },
            title = {
                Text(if (peer.status == PeerTrustStatus.Pending) "处理设备信任" else "管理设备")
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(
                        if (peer.status == PeerTrustStatus.Pending) {
                            "这是刚刚连接到的设备。你可以直接确认信任、拒绝，或者先修改备注。"
                        } else {
                            "你可以修改备注，或调整这个设备的信任状态。"
                        },
                    )
                    Text("当前状态：${peer.status.displayLabel()}")
                    Text("颜文字指纹：${peer.kaomojiFingerprint}")
                    Text("摘要：${fingerprintHex(peer.fingerprint)}")
                    OutlinedTextField(
                        value = peerNoteDraft,
                        onValueChange = { peerNoteDraft = it },
                        label = { Text("备注") },
                        modifier = Modifier.fillMaxWidth(),
                    )
                    FilledTonalButton(
                        onClick = { onUpdatePeerNote(peer.id, peerNoteDraft) },
                        enabled = !uiState.isManagingPeer,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text("保存备注")
                    }
                    if (peer.status == PeerTrustStatus.Pending) {
                        Button(
                            onClick = {
                                onTrustPeer(peer.publicKey, peerNoteDraft)
                                onDismissPendingTrustPrompt()
                                activePeer = null
                                peerNoteDraft = ""
                            },
                            enabled = !uiState.isManagingPeer,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text("设为可信")
                        }
                        FilledTonalButton(
                            onClick = {
                                onSetPeerStatus(peer.id, PeerTrustStatus.Revoked)
                                onDismissPendingTrustPrompt()
                                activePeer = null
                                peerNoteDraft = ""
                            },
                            enabled = !uiState.isManagingPeer,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text("拒绝这个设备")
                        }
                    } else {
                        val targetStatus = if (peer.status == PeerTrustStatus.Trusted) {
                            PeerTrustStatus.Revoked
                        } else {
                            PeerTrustStatus.Trusted
                        }
                        val actionLabel = if (peer.status == PeerTrustStatus.Trusted) {
                            "撤销信任"
                        } else {
                            "设为可信"
                        }
                        FilledTonalButton(
                            onClick = { onSetPeerStatus(peer.id, targetStatus) },
                            enabled = !uiState.isManagingPeer,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(actionLabel)
                        }
                    }
                    TextButton(
                        onClick = {
                            onDeletePeer(peer.id)
                            if (uiState.pendingTrustPeer?.id == peer.id) {
                                onDismissPendingTrustPrompt()
                            }
                            activePeer = null
                            peerNoteDraft = ""
                        },
                        enabled = !uiState.isManagingPeer,
                    ) {
                        Text("删除这个设备")
                    }
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        if (uiState.pendingTrustPeer?.id == peer.id) {
                            onDismissPendingTrustPrompt()
                        }
                        activePeer = null
                        peerNoteDraft = ""
                    },
                ) {
                    Text("关闭")
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
                    session.peerFingerprint?.takeIf { it.isNotEmpty() }?.let { fingerprint ->
                        SyncDetailBlock("对端指纹", fingerprintHex(fingerprint))
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
private fun SyncOverviewCard(
    status: String,
    protocol: String,
    backend: String,
    port: Int?,
    localAddresses: List<String>,
    isListening: Boolean,
    onRefresh: () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Filled.Wifi,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
                Spacer(modifier = Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "监听状态",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = status,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                FilledTonalButton(onClick = onRefresh) {
                    Text("刷新")
                }
            }

            SyncInfoLine("协议", protocol)
            SyncInfoLine("实现", backend)
            SyncInfoLine("监听端口", port?.toString() ?: "未分配")
            SyncInfoLine(
                "局域网地址",
                localAddresses.takeIf { it.isNotEmpty() }?.joinToString(", ") ?: "未获取到局域网地址",
            )
            SyncInfoLine("状态", if (isListening) "已监听" else "未监听")
        }
    }
}

@Composable
private fun IdentitySection(identity: LocalIdentity?) {
    SectionTitle(title = "本机身份", subtitle = "展示当前设备的身份与签名密钥，方便人工核对和复制")

    if (identity == null) {
        EmptySectionCard("正在读取本机身份信息")
        return
    }

    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        BoxWithConstraints {
            val useWideLayout = maxWidth >= 760.dp

            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Filled.Key,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                    )
                    Spacer(modifier = Modifier.width(12.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "本机密钥材料",
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = if (useWideLayout) {
                                "平板上横向展开展示，手机上自动切成紧凑卡片"
                            } else {
                                "当前使用紧凑展示，只保留 Base64 公钥与识别码"
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    SuggestionChip(
                        onClick = {},
                        enabled = false,
                        label = { Text("本机") },
                    )
                }

                if (useWideLayout) {
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        IdentityKeyPanel(
                            modifier = Modifier.weight(1f),
                            title = "身份密钥",
                            subtitle = identity.identity.algorithm,
                            displayKey = identity.identity.displayPublicKeyBase64,
                            recognitionCode = identity.identity.kaomojiFingerprint,
                            compact = false,
                        )
                        IdentityKeyPanel(
                            modifier = Modifier.weight(1f),
                            title = "签名密钥",
                            subtitle = identity.signing.algorithm,
                            displayKey = identity.signing.displayPublicKeyBase64,
                            recognitionCode = identity.signing.kaomojiFingerprint,
                            compact = false,
                        )
                    }
                } else {
                    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        IdentityKeyPanel(
                            title = "身份密钥",
                            subtitle = identity.identity.algorithm,
                            displayKey = identity.identity.displayPublicKeyBase64,
                            recognitionCode = identity.identity.kaomojiFingerprint,
                            compact = true,
                        )
                        IdentityKeyPanel(
                            title = "签名密钥",
                            subtitle = identity.signing.algorithm,
                            displayKey = identity.signing.displayPublicKeyBase64,
                            recognitionCode = identity.signing.kaomojiFingerprint,
                            compact = true,
                        )
                    }
                }
            }
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
                EmptySectionCard("还没有任何可用连接，等待局域网发现或手动添加地址和端口")
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
    onManagePeer: (PeerRecord) -> Unit,
) {
    SectionTitle(title = "设备列表", subtitle = "包含已信任、待确认和已撤销的对端公钥")

    if (peers.isEmpty()) {
        EmptySectionCard("还没有发现任何对端设备")
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
                    onManagePeer = onManagePeer,
                )
            }
        }
    }
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
private fun IdentityKeyPanel(
    modifier: Modifier = Modifier,
    title: String,
    subtitle: String,
    displayKey: String,
    recognitionCode: String,
    compact: Boolean,
) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(if (compact) 16.dp else 20.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
    ) {
        Column(
            modifier = Modifier.padding(if (compact) 12.dp else 14.dp),
            verticalArrangement = Arrangement.spacedBy(if (compact) 8.dp else 10.dp),
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Surface(
                shape = RoundedCornerShape(16.dp),
                color = MaterialTheme.colorScheme.surface,
            ) {
                Column(
                    modifier = Modifier.padding(if (compact) 10.dp else 12.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Text(
                        text = "公钥 Base64",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        text = displayKey,
                        style = if (compact) {
                            MaterialTheme.typography.bodySmall
                        } else {
                            MaterialTheme.typography.bodyMedium
                        },
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            SuggestionChip(
                onClick = {},
                enabled = false,
                label = { Text("识别码 $recognitionCode") },
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
private fun PeerRow(
    peer: PeerRecord,
    isManagingPeer: Boolean,
    onManagePeer: (PeerRecord) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 14.dp)
            .clickable {
                onManagePeer(peer)
            },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = Icons.Filled.Devices,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = peer.note ?: "未命名设备",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "${peer.algorithm} · ${peer.kaomojiFingerprint}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = peer.status.displayLabel(),
                style = MaterialTheme.typography.labelLarge,
                color = peer.status.color(),
            )
        }

        FilledTonalButton(
            onClick = { onManagePeer(peer) },
            enabled = !isManagingPeer,
        ) {
            Icon(
                imageVector = if (peer.status == PeerTrustStatus.Pending) {
                    Icons.Filled.Check
                } else {
                    Icons.Filled.Key
                },
                contentDescription = null,
                modifier = Modifier.size(16.dp),
            )
            Spacer(modifier = Modifier.width(6.dp))
            Text(if (peer.status == PeerTrustStatus.Pending) "处理" else "管理")
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
        )
    }
}

@Composable
private fun PeerTrustStatus.color() = when (this) {
    PeerTrustStatus.Pending -> MaterialTheme.colorScheme.primary
    PeerTrustStatus.Trusted -> MaterialTheme.colorScheme.tertiary
    PeerTrustStatus.Retired -> MaterialTheme.colorScheme.secondary
    PeerTrustStatus.Revoked -> MaterialTheme.colorScheme.error
}

@Composable
private fun SyncConnectionStatus.color() = when (this) {
    SyncConnectionStatus.Idle -> MaterialTheme.colorScheme.secondary
    SyncConnectionStatus.Connecting -> MaterialTheme.colorScheme.primary
    SyncConnectionStatus.AwaitingTrust -> MaterialTheme.colorScheme.tertiary
    SyncConnectionStatus.Connected -> MaterialTheme.colorScheme.tertiary
    SyncConnectionStatus.Failed -> MaterialTheme.colorScheme.error
}

private fun PeerTrustStatus.displayLabel(): String = when (this) {
    PeerTrustStatus.Pending -> "待确认"
    PeerTrustStatus.Trusted -> "已信任"
    PeerTrustStatus.Retired -> "已停用"
    PeerTrustStatus.Revoked -> "已撤销"
}

private fun SyncStatus.displayLabel(): String = when (this) {
    SyncStatus.Completed -> "已完成"
    SyncStatus.PendingTrust -> "待信任"
    SyncStatus.Failed -> "失败"
}

private fun SyncSessionRole.displayLabel(): String = when (this) {
    SyncSessionRole.Initiator -> "主动发起"
    SyncSessionRole.Listener -> "被动接收"
}

private fun fingerprintHex(bytes: ByteArray): String =
    bytes.joinToString(":") { byte -> "%02X".format(byte.toInt() and 0xFF) }

private fun formatTimestamp(timestampMs: Long): String =
    DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")
        .format(Instant.ofEpochMilli(timestampMs).atZone(ZoneId.systemDefault()))
