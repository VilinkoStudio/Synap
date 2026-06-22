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
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.CellTower
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import com.synap.app.data.model.UnifiedDevice
import com.synap.app.data.model.UnifiedDeviceStatus
import com.synap.app.data.model.SyncSessionRecord
import com.synap.app.data.model.SyncSessionRole
import com.synap.app.data.model.SyncStatus
import com.synap.app.ui.viewmodel.SyncUiState
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.concurrent.CancellationException
import kotlinx.coroutines.launch

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
    var selectedTab by remember { mutableStateOf(0) }

    val localPublicKeyBase64 = uiState.localIdentity?.identity?.displayPublicKeyBase64
    val localSigningBase64 = uiState.localIdentity?.signing?.displayPublicKeyBase64
    val localSigningPublicKey = uiState.localIdentity?.signing?.publicKey
    val localAddresses = uiState.listenerState.localAddresses.map { it.lowercase().trim() }.toSet()
    val localPort = uiState.listenerState.listenPort
    val localFingerprint = uiState.localIdentity?.signing?.kaomojiFingerprint

    // 过滤掉本机设备
    val filteredUnifiedDevices = uiState.unifiedDevices.filter { device ->
        val isSelfPeer = device.peer?.let { peer ->
            peer.displayPublicKeyBase64 == localPublicKeyBase64 ||
                peer.displayPublicKeyBase64 == localSigningBase64
        } ?: false
        val isSelfDiscovered = device.discovered?.let { peer ->
            val isSameHost = localPort != null && localAddresses.any { addr ->
                peer.host.lowercase().trim() == addr
            } && peer.port == localPort
            val isSameFingerprint = localFingerprint != null &&
                peer.serviceName.contains(localFingerprint)
            isSameHost || isSameFingerprint
        } ?: false
        !isSelfPeer && !isSelfDiscovered
    }
    // 保留手动连接过滤
    val filteredPeers = uiState.peers.filter { peer ->
        peer.displayPublicKeyBase64 != localPublicKeyBase64 && peer.displayPublicKeyBase64 != localSigningBase64
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
                translationX = backProgress * 64.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
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
                    FilledTonalButton(
                        onClick = { showListeningInfoDialog = true },
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Filled.CellTower,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text = if (uiState.listenerState.isListening) "正在监听设备" else "未监听",
                            style = MaterialTheme.typography.labelSmall
                        )
                    }
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
                .padding(innerPadding),
        ) {
            val tabTitles = listOf("局域网同步", "服务器同步", "同步记录")
            val coroutineScope = rememberCoroutineScope()
            val pagerState = rememberPagerState(pageCount = { tabTitles.size })

            LaunchedEffect(pagerState.currentPage) {
                selectedTab = pagerState.currentPage
            }

            TabRow(
                selectedTabIndex = selectedTab,
            ) {
                tabTitles.forEachIndexed { index, title ->
                    Tab(
                        selected = selectedTab == index,
                        onClick = {
                            selectedTab = index
                            coroutineScope.launch { pagerState.animateScrollToPage(index) }
                        },
                        text = { Text(title) },
                    )
                }
            }

            HorizontalPager(
                state = pagerState,
                modifier = Modifier.fillMaxSize(),
            ) { page ->
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .verticalScroll(rememberScrollState())
                        .padding(horizontal = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Spacer(modifier = Modifier.height(4.dp))

                    if (page == 0) {
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
                    }

                    when (page) {
                        0 -> {
                            IdentitySection(identity = uiState.localIdentity)
                            DeviceSection(
                                devices = filteredUnifiedDevices,
                                isPairing = uiState.isPairing,
                                isManagingPeer = uiState.isManagingPeer,
                                onAddConnection = { showAddConnectionDialog = true },
                                onPair = { host, port -> onPairDiscoveredPeer(host, port) },
                                onTrustPeer = onTrustPeer,
                                onUpdatePeerNote = onUpdatePeerNote,
                                onDeletePeer = onDeletePeer,
                                onSetPeerStatus = onSetPeerStatus,
                                onDismissPendingTrustPrompt = onDismissPendingTrustPrompt,
                            )
                            // 保留手动连接列表
                            if (uiState.connections.isNotEmpty()) {
                                ConnectionSection(
                                    discoveredPeers = emptyList(),
                                    connections = uiState.connections,
                                    isPairing = uiState.isPairing,
                                    onAddConnection = { showAddConnectionDialog = true },
                                    onDeleteConnection = onDeleteConnection,
                                    onPairConnection = onPairConnection,
                                    onPairDiscoveredPeer = onPairDiscoveredPeer,
                                )
                            }
                        }
                        1 -> {
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
                        }
                        2 -> {
                            SyncStatsSection(
                                sessions = uiState.recentSyncSessions,
                                onOpenSession = { session -> selectedSyncSession = session },
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(24.dp))
                }
            }
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
                    SyncInfoLine("状态", if (uiState.listenerState.isListening) "正在监听设备" else "未监听")
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
    SectionTitle(title = "本机签名密钥", subtitle = "使用Ed25519算法生成唯一数字签名密钥，用于连接设备时的身份确认")

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
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "可用连接",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.weight(1f),
        )
        FilledTonalButton(
            onClick = onAddConnection,
            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = Icons.Filled.Add,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
            )
            Spacer(modifier = Modifier.width(6.dp))
            Text("添加连接")
        }
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Column {
            if (discoveredPeers.isEmpty() && connections.isEmpty()) {
                EmptySectionCard("暂无可用连接。如果无法搜索到您的设备，请检查是否和其他设备处于同一局域网下，或者您可以尝试点击\u201C添加连接\u201D手动添加设备")
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
private fun DeviceSection(
    devices: List<UnifiedDevice>,
    isPairing: Boolean,
    isManagingPeer: Boolean,
    onAddConnection: () -> Unit,
    onPair: (String, Int) -> Unit,
    onTrustPeer: (ByteArray, String?) -> Unit,
    onUpdatePeerNote: (String, String?) -> Unit,
    onDeletePeer: (String) -> Unit,
    onSetPeerStatus: (String, PeerTrustStatus) -> Unit,
    onDismissPendingTrustPrompt: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "设备列表",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.weight(1f),
        )
        FilledTonalButton(
            onClick = onAddConnection,
            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = Icons.Filled.Add,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
            )
            Spacer(modifier = Modifier.width(6.dp))
            Text("手动添加")
        }
    }

    if (devices.isEmpty()) {
        EmptySectionCard("暂未发现设备。请确保设备在同一局域网下，或点击“手动添加”输入地址。")
        return
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp)),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Column {
            devices.forEachIndexed { index, device ->
                if (index > 0) {
                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )
                }
                DeviceRow(
                    device = device,
                    isPairing = isPairing,
                    isManagingPeer = isManagingPeer,
                    onPair = { onPair(device.discovered!!.host, device.discovered.port) },
                    onTrustPeer = { onTrustPeer(device.peer!!.publicKey, it) },
                    onUpdatePeerNote = { note -> onUpdatePeerNote(device.peer!!.id, note) },
                    onDeletePeer = { onDeletePeer(device.peer!!.id) },
                    onSetPeerStatus = { status -> onSetPeerStatus(device.peer!!.id, status) },
                    onDismissPendingTrustPrompt = onDismissPendingTrustPrompt,
                )
            }
        }
    }
}

@Composable
private fun DeviceRow(
    device: UnifiedDevice,
    isPairing: Boolean,
    isManagingPeer: Boolean,
    onPair: () -> Unit,
    onTrustPeer: (String?) -> Unit,
    onUpdatePeerNote: (String?) -> Unit,
    onDeletePeer: () -> Unit,
    onSetPeerStatus: (PeerTrustStatus) -> Unit,
    onDismissPendingTrustPrompt: () -> Unit,
) {
    var expanded by remember { mutableStateOf(device.status == UnifiedDeviceStatus.Pending) }
    var noteDraft by remember(device.peer?.note) { mutableStateOf(device.peer?.note.orEmpty()) }
    var isEditingNote by remember { mutableStateOf(false) }
    var showDeleteConfirm by remember { mutableStateOf(false) }

    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text("删除设备") },
            text = { Text("确定要删除该设备吗？删除后需要重新配对才能同步。") },
            confirmButton = {
                Button(onClick = {
                    showDeleteConfirm = false
                    onDeletePeer()
                }) { Text("删除") }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) { Text("取消") }
            },
        )
    }

    val avatarBitmap = remember(device.peer?.avatarPng) {
        device.peer?.avatarPng?.let {
            BitmapFactory.decodeByteArray(it, 0, it.size)
        }
    }

    val statusLabel = when (device.status) {
        UnifiedDeviceStatus.Pending -> "待信任"
        UnifiedDeviceStatus.NewOnline -> "新设备"
        UnifiedDeviceStatus.TrustedOnline -> "在线"
        UnifiedDeviceStatus.TrustedOffline -> "离线"
        UnifiedDeviceStatus.Revoked -> "已撤销"
    }
    val statusColor = when (device.status) {
        UnifiedDeviceStatus.Pending -> MaterialTheme.colorScheme.tertiary
        UnifiedDeviceStatus.NewOnline -> MaterialTheme.colorScheme.primary
        UnifiedDeviceStatus.TrustedOnline -> MaterialTheme.colorScheme.primary
        UnifiedDeviceStatus.TrustedOffline -> MaterialTheme.colorScheme.onSurfaceVariant
        UnifiedDeviceStatus.Revoked -> MaterialTheme.colorScheme.error
    }
    val isOnline = device.status == UnifiedDeviceStatus.TrustedOnline ||
        device.status == UnifiedDeviceStatus.NewOnline

    Column(modifier = Modifier.fillMaxWidth()) {
        // 主行
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clickable(enabled = device.peer != null) { expanded = !expanded }
                .padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // 头像 / 在线指示
            if (avatarBitmap != null) {
                Box {
                    PublicKeyAvatar(
                        bitmap = avatarBitmap,
                        contentDescription = "设备头像",
                        modifier = Modifier
                            .size(44.dp)
                            .clip(RoundedCornerShape(14.dp)),
                    )
                    if (isOnline) {
                        Box(
                            modifier = Modifier
                                .size(12.dp)
                                .align(Alignment.BottomEnd)
                                .background(
                                    MaterialTheme.colorScheme.primary,
                                    shape = RoundedCornerShape(6.dp),
                                ),
                        )
                    }
                }
            } else {
                Box(contentAlignment = Alignment.BottomEnd) {
                    Icon(
                        imageVector = Icons.Filled.Sync,
                        contentDescription = null,
                        tint = statusColor,
                        modifier = Modifier
                            .size(44.dp)
                            .padding(8.dp),
                    )
                    if (isOnline) {
                        Box(
                            modifier = Modifier
                                .size(12.dp)
                                .align(Alignment.BottomEnd)
                                .background(
                                    MaterialTheme.colorScheme.primary,
                                    shape = RoundedCornerShape(6.dp),
                                ),
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.width(12.dp))

            // 名称 + 详情
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = device.displayName.removePrefix("Synap·").removePrefix("Synap-"),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                if (device.peer != null) {
                    Text(
                        text = device.peer.displayPublicKeyBase64,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else if (device.discovered != null) {
                    Text(
                        text = "${device.discovered.host}:${device.discovered.port}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            // 状态标签
            Text(
                text = statusLabel,
                style = MaterialTheme.typography.labelLarge,
                color = statusColor,
            )

            Spacer(modifier = Modifier.width(8.dp))

            // 操作按钮
            when (device.status) {
                UnifiedDeviceStatus.NewOnline -> {
                    FilledTonalButton(
                        onClick = onPair,
                        enabled = !isPairing,
                    ) { Text("配对") }
                }
                UnifiedDeviceStatus.Pending -> {
                    // 按钮在展开区域
                    Icon(
                        imageVector = if (expanded) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                UnifiedDeviceStatus.TrustedOnline -> {
                    FilledTonalButton(
                        onClick = onPair,
                        enabled = !isPairing,
                    ) { Text("同步") }
                }
                UnifiedDeviceStatus.TrustedOffline -> {
                    Icon(
                        imageVector = if (expanded) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                UnifiedDeviceStatus.Revoked -> {
                    Icon(
                        imageVector = if (expanded) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }

        // 展开区域（仅已信任设备）
        if (device.peer != null) {
            AnimatedVisibility(
                visible = expanded,
                enter = expandVertically(),
                exit = shrinkVertically(),
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 16.dp, end = 16.dp, bottom = 14.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    if (device.status == UnifiedDeviceStatus.Pending) {
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
                            ) { Text("信任") }
                            FilledTonalButton(
                                onClick = {
                                    onSetPeerStatus(PeerTrustStatus.Revoked)
                                    onDismissPendingTrustPrompt()
                                },
                                enabled = !isManagingPeer,
                                modifier = Modifier.weight(1f),
                            ) { Text("拒绝") }
                        }
                    } else {
                        // 备注编辑
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            if (isEditingNote) {
                                OutlinedTextField(
                                    value = noteDraft,
                                    onValueChange = { noteDraft = it },
                                    label = { Text("备注") },
                                    modifier = Modifier.weight(1f),
                                    singleLine = true,
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Button(onClick = {
                                    onUpdatePeerNote(noteDraft)
                                    isEditingNote = false
                                }) { Text("保存") }
                            } else {
                                Text(
                                    text = device.peer.note ?: "未设置备注",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.weight(1f),
                                )
                                TextButton(onClick = { isEditingNote = true }) {
                                    Text("编辑")
                                }
                            }
                        }

                        // 信任状态切换
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            if (device.peer.status == PeerTrustStatus.Trusted) {
                                OutlinedButton(
                                    onClick = { onSetPeerStatus(PeerTrustStatus.Revoked) },
                                    enabled = !isManagingPeer,
                                    modifier = Modifier.weight(1f),
                                ) { Text("撤销信任") }
                            } else if (device.peer.status == PeerTrustStatus.Revoked) {
                                Button(
                                    onClick = { onSetPeerStatus(PeerTrustStatus.Trusted) },
                                    enabled = !isManagingPeer,
                                    modifier = Modifier.weight(1f),
                                ) { Text("恢复信任") }
                            }
                            OutlinedButton(
                                onClick = { showDeleteConfirm = true },
                                enabled = !isManagingPeer,
                                colors = ButtonDefaults.outlinedButtonColors(
                                    contentColor = MaterialTheme.colorScheme.error,
                                ),
                                modifier = Modifier.weight(1f),
                            ) { Text("删除") }
                        }
                    }
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
    var savedBaseUrl by remember { mutableStateOf(baseUrl) }
    var savedApiKey by remember { mutableStateOf(apiKey) }
    var showSaveConfirm by remember { mutableStateOf(false) }

    val isUnsaved = baseUrl != savedBaseUrl || apiKey != savedApiKey
    val canOperate = !isSyncing && baseUrl.isNotBlank() && apiKey.isNotBlank()

    SectionTitle(title = "服务器", subtitle = "使用服务器同步数据，可跨网络使用，请勿将数据保存在不信任的服务器上。同步前需填写服务器地址和API密钥，并在设备列表将需要同步的设备添加为\u201C可信任设备\u201D")

    if (showSaveConfirm) {
        AlertDialog(
            onDismissRequest = { showSaveConfirm = false },
            title = { Text("确认修改服务器信息") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("修改前：", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary, modifier = Modifier.fillMaxWidth(), textAlign = TextAlign.Center)
                    SyncInfoLine("服务器地址：", savedBaseUrl.ifBlank { "（空）" })
                    SyncInfoLine("API 密钥：", savedApiKey.ifBlank { "（空）" })
                    Spacer(modifier = Modifier.height(4.dp))
                    Text("修改后：", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary, modifier = Modifier.fillMaxWidth(), textAlign = TextAlign.Center)
                    SyncInfoLine("服务器地址：", baseUrl.ifBlank { "（空）" })
                    SyncInfoLine("API 密钥：", apiKey.ifBlank { "（空）" })
                }
            },
            confirmButton = {
                Button(onClick = {
                    showSaveConfirm = false
                    savedBaseUrl = baseUrl
                    savedApiKey = apiKey
                    onSave()
                }) { Text("保存") }
            },
            dismissButton = {
                TextButton(onClick = { showSaveConfirm = false }) { Text("取消") }
            }
        )
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
            OutlinedTextField(
                value = baseUrl,
                onValueChange = onBaseUrlChange,
                label = { Text("服务器地址") },
                placeholder = { Text("http://relay.example.com:8080") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )
            OutlinedTextField(
                value = apiKey,
                onValueChange = onApiKeyChange,
                label = { Text("API 密钥") },
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
            if (isUnsaved) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    OutlinedButton(
                        onClick = {
                            onBaseUrlChange(savedBaseUrl)
                            onApiKeyChange(savedApiKey)
                        },
                        enabled = !isSyncing,
                        modifier = Modifier.weight(1f),
                    ) {
                        Text("取消")
                    }
                    Button(
                        onClick = {
                            val wasEmpty = savedBaseUrl.isBlank() && savedApiKey.isBlank()
                            if (wasEmpty) {
                                savedBaseUrl = baseUrl
                                savedApiKey = apiKey
                                onSave()
                            } else {
                                showSaveConfirm = true
                            }
                        },
                        enabled = !isSyncing,
                        modifier = Modifier.weight(1f),
                    ) {
                        Text("保存")
                    }
                }
            } else if (canOperate) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Button(
                        onClick = onFetch,
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(Icons.Filled.ArrowDownward, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(modifier = Modifier.width(4.dp))
                        Text("拉取")
                    }
                    Button(
                        onClick = onPush,
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(Icons.Filled.ArrowUpward, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(modifier = Modifier.width(4.dp))
                        Text("推送")
                    }
                }
            }
        }
    }

    Spacer(modifier = Modifier.height(4.dp))
    Text(
        text = "推荐的服务器",
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 4.dp),
    )
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .clickable {
                onBaseUrlChange("http://relay.synap.vilinko.com")
                onApiKeyChange("default-key")
            },
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "Vilinko官方服务器",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = "官方",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier
                            .background(MaterialTheme.colorScheme.primaryContainer, RoundedCornerShape(4.dp))
                            .padding(horizontal = 6.dp, vertical = 2.dp),
                    )
                }
                Text(
                    text = "http://relay.synap.vilinko.com",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
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
                text = peer.displayName.removePrefix("Synap·").removePrefix("Synap-"),
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
private fun SyncStatsSection(
    sessions: List<SyncSessionRecord>,
    onOpenSession: (SyncSessionRecord) -> Unit,
) {
    val latest = sessions.firstOrNull()
    if (latest == null) {
        Text(
            text = "上次同步",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(start = 4.dp),
        )
        EmptySectionCard("还没有任何同步记录")
        return
    }

    Text(
        text = "上次同步",
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 4.dp),
    )
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
            SyncInfoLine("时间", formatTimestamp(latest.finishedAtMs.toLong()))
            SyncInfoLine("结果", latest.status.displayLabel())
            SyncInfoLine("方向", latest.role.displayLabel())
            SyncInfoLine("设备", latest.peerLabel ?: "未命名设备")
            SyncInfoLine("应用记录", latest.recordsApplied.toString())
            SyncInfoLine("发送/接收", "${latest.recordsSent}/${latest.recordsReceived}")
            SyncInfoLine("耗时", "${latest.durationMs} ms")
        }
    }

    if (sessions.size > 1) {
        Text(
            text = "历史同步记录",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(start = 4.dp),
        )
        Surface(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(16.dp)),
            color = MaterialTheme.colorScheme.surfaceVariant,
        ) {
            Column {
                sessions.drop(1).take(5).forEachIndexed { index, session ->
                    if (index > 0) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        )
                    }
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onOpenSession(session) }
                            .padding(horizontal = 16.dp, vertical = 12.dp),
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
            style = MaterialTheme.typography.bodySmall,
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
