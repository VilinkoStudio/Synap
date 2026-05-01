package com.synap.app.ui.screens

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
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
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.DeleteOutline
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
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.ui.graphics.Color
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
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontWeight
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

    // ========== 新增：控制监听状态弹窗的开关 ==========
    var showListeningInfoDialog by remember { mutableStateOf(false) }

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

    activePeer?.let { peer ->
        val peerAvatarBitmap = remember(peer.avatarPng) {
            BitmapFactory.decodeByteArray(peer.avatarPng, 0, peer.avatarPng.size)
        }
        AlertDialog(
            onDismissRequest = {
                if (uiState.pendingTrustPeer?.id == peer.id) {
                    onDismissPendingTrustPrompt()
                }
                activePeer = null
                peerNoteDraft = ""
            },
            icon = {
                PublicKeyAvatar(
                    bitmap = peerAvatarBitmap,
                    contentDescription = "设备头像",
                    modifier = Modifier
                        .size(40.dp)
                        .clip(RoundedCornerShape(12.dp)),
                )
            },
            title = {
                Text(if (peer.status == PeerTrustStatus.Pending) "处理设备信任" else "管理设备")
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text(
                        if (peer.status == PeerTrustStatus.Pending) {
                            "这是刚刚连接到的设备。你可以直接确认信任、拒绝，或者先修改备注。"
                        } else {
                            "你可以修改备注，或调整这个设备的信任状态。"
                        },
                    )
                    Text("当前状态：${peer.status.displayLabel()}")

                    // ========== 核心修改 2：包装的设备信息 ==========
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(
                            modifier = Modifier.padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(12.dp)
                        ) {
                            Text("设备信息", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                PublicKeyAvatar(
                                    bitmap = peerAvatarBitmap,
                                    contentDescription = "设备头像",
                                    modifier = Modifier
                                        .size(56.dp)
                                        .clip(RoundedCornerShape(16.dp)),
                                )
                                Column(
                                    modifier = Modifier.weight(1f),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Text(
                                        text = peer.note ?: "未命名设备",
                                        style = MaterialTheme.typography.titleSmall,
                                        color = MaterialTheme.colorScheme.onSurface,
                                    )
                                    Text(
                                        text = peer.algorithm,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                            Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                SyncInfoLine("设备摘要", fingerprintHex(peer.fingerprint))
                            }
                        }
                    }

                    // ========== 核心修改 3：备注输入框与保存按钮排成一行 ==========
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        OutlinedTextField(
                            value = peerNoteDraft,
                            onValueChange = { peerNoteDraft = it },
                            label = { Text("备注") },
                            modifier = Modifier.weight(1f),
                            singleLine = true
                        )
                        FilledTonalButton(
                            onClick = { onUpdatePeerNote(peer.id, peerNoteDraft) },
                            enabled = !uiState.isManagingPeer,
                        ) {
                            Text("保存")
                        }
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

                    // ========== 核心修改 4：居中且带有警示色的删除按钮 ==========
                    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
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
                            Text("删除这个设备", color = MaterialTheme.colorScheme.error)
                        }
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
private fun IdentitySection(identity: LocalIdentity?) {
    SectionTitle(title = "此设备秘钥信息", subtitle = "根据当前设备的数据生成唯一身份和签名密钥")

    if (identity == null) {
        EmptySectionCard("正在读取此设备信息")
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
                            text = "身份密钥与签名密钥统一展示在同一张卡片中",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                IdentityKeysPanel(
                    identity = identity,
                    modifier = if (useWideLayout) Modifier.fillMaxWidth() else Modifier,
                )
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
private fun IdentityKeysPanel(
    identity: LocalIdentity,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
    ) {
        Column(
            modifier = Modifier.padding(vertical = 8.dp),
        ) {
            IdentityKeyRow(
                title = "身份密钥",
                algorithm = identity.identity.algorithm,
                avatarPng = identity.identity.avatarPng,
                displayKey = identity.identity.displayPublicKeyBase64,
                showLocalBadge = true,
            )
            HorizontalDivider(
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                modifier = Modifier.padding(horizontal = 16.dp),
            )
            IdentityKeyRow(
                title = "签名密钥",
                algorithm = identity.signing.algorithm,
                avatarPng = identity.signing.avatarPng,
                displayKey = identity.signing.displayPublicKeyBase64,
                showLocalBadge = false,
            )
        }
    }
}

@Composable
private fun IdentityKeyRow(
    title: String,
    algorithm: String,
    avatarPng: ByteArray,
    displayKey: String,
    showLocalBadge: Boolean,
) {
    val avatarBitmap = remember(avatarPng) {
        BitmapFactory.decodeByteArray(avatarPng, 0, avatarPng.size)
    }

    ListItem(
        colors = ListItemDefaults.colors(
            containerColor = Color.Transparent,
        ),
        tonalElevation = 0.dp,
        leadingContent = {
            PublicKeyAvatar(
                bitmap = avatarBitmap,
                contentDescription = "$title 头像",
                modifier = Modifier
                    .size(52.dp)
                    .clip(RoundedCornerShape(14.dp)),
            )
        },
        headlineContent = {
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
            )
        },
        supportingContent = {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    text = "算法 $algorithm",
                    style = MaterialTheme.typography.bodySmall,
                )
                Text(
                    text = displayKey,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        trailingContent = {
            if (showLocalBadge) {
                Surface(
                    shape = RoundedCornerShape(999.dp),
                    color = MaterialTheme.colorScheme.surfaceContainerHighest,
                ) {
                    Text(
                        text = "本机",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                    )
                }
            }
        },
    )
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
private fun PeerRow(
    peer: PeerRecord,
    isManagingPeer: Boolean,
    onManagePeer: (PeerRecord) -> Unit,
) {
    val avatarBitmap = remember(peer.avatarPng) {
        BitmapFactory.decodeByteArray(peer.avatarPng, 0, peer.avatarPng.size)
    }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 14.dp)
            .clickable {
                onManagePeer(peer)
            },
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
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = peer.algorithm,
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
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f)
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
