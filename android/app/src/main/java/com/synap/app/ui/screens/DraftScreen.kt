package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberSwipeToDismissBoxState
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.data.service.DraftRecord
import com.synap.app.data.service.DraftStore
import com.synap.app.ui.util.NoteColorUtil
import kotlinx.coroutines.CancellationException
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DraftScreen(
    onNavigateBack: () -> Unit,
    onDraftClick: (DraftRecord) -> Unit,
) {
    val context = LocalContext.current
    val draftStore = remember { DraftStore(context) }
    var drafts by remember { mutableStateOf(draftStore.list()) }
    var showMessage by remember { mutableStateOf(true) }
    var showClearDialog by remember { mutableStateOf(false) }

    // 每次重组时读取最新容量
    val capacity = draftStore.getCapacity()
    val currentCount = drafts.size

    fun refreshDrafts() {
        drafts = draftStore.list()
    }

    // 自动刷新草稿箱数据
    LaunchedEffect(Unit) {
        refreshDrafts()
    }

    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress
            }
            onNavigateBack()
        } catch (e: CancellationException) {
            backProgress = 0f
        }
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            .graphicsLayer {
                translationX = backProgress * 64.dp.toPx()
                transformOrigin = TransformOrigin(1f, 0.5f)
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            },
        topBar = {
            TopAppBar(
                title = { Text("草稿箱") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    Text(
                        text = "$currentCount/$capacity",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    IconButton(onClick = { showClearDialog = true }) {
                        Icon(Icons.Filled.DeleteSweep, contentDescription = "清空草稿箱")
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            // 提示信息
            if (showMessage) {
                Surface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    shape = RoundedCornerShape(12.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            text = "这里存放还未保存的笔记，向右滑动笔记卡片即可继续编辑，向左滑动即可永久删除。您可以在设置中调整草稿箱的容量，若草稿箱已满，会按时间顺序移除最早的笔记。草稿箱仅用于笔记数据临时存放，注意及时保存数据。",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                        ) {
                            TextButton(onClick = { showMessage = false }) {
                                Text("永久关闭")
                            }
                            Button(onClick = { showMessage = false }) {
                                Text("关闭")
                            }
                        }
                    }
                }
            }

            if (drafts.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = "草稿箱为空",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            } else {
                LazyVerticalStaggeredGrid(
                    columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalItemSpacing = 12.dp,
                ) {
                    itemsIndexed(drafts) { index, draft ->
                        DraftCard(
                            draft = draft,
                            onClick = { onDraftClick(draft) },
                            onDelete = {
                                draftStore.delete(draft.id)
                                refreshDrafts()
                            },
                        )
                    }
                }
            }
        }
    }

    // 清空草稿箱确认弹窗
    if (showClearDialog) {
        AlertDialog(
            onDismissRequest = { showClearDialog = false },
            title = { Text("确认删除") },
            text = { Text("是否确认删除草稿箱内的全部${currentCount}条笔记") },
            confirmButton = {
                TextButton(
                    onClick = {
                        draftStore.clear()
                        refreshDrafts()
                        showClearDialog = false
                    },
                    colors = ButtonDefaults.textButtonColors(
                        contentColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Text("删除")
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDialog = false }) {
                    Text("取消")
                }
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DraftCard(
    draft: DraftRecord,
    onClick: () -> Unit,
    onDelete: () -> Unit,
) {
    // 【核心修复 1】：使用底层触摸 API，记录用户是否处于长按/滑动未松手状态
    var isPressed by remember { mutableStateOf(false) }

    val dismissState = rememberSwipeToDismissBoxState(
        confirmValueChange = { dismissValue ->
            // 仅放行状态变化（让底色变色），绝不在这里执行删除逻辑！
            when (dismissValue) {
                SwipeToDismissBoxValue.StartToEnd -> true
                SwipeToDismissBoxValue.EndToStart -> true
                SwipeToDismissBoxValue.Settled -> true
            }
        },
        // 滑动 5% 即可触发状态（背景变色）
        positionalThreshold = { totalDistance -> totalDistance * 0.05f }
    )

    // 【核心修复 3】：只有等用户彻底松手（isPressed = false）并且状态已切换，才执行真正动作
    LaunchedEffect(dismissState.currentValue, isPressed) {
        if (!isPressed) {
            when (dismissState.currentValue) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    onClick()
                    dismissState.snapTo(SwipeToDismissBoxValue.Settled)
                }
                SwipeToDismissBoxValue.EndToStart -> {
                    onDelete()
                    dismissState.snapTo(SwipeToDismissBoxValue.Settled)
                }
                SwipeToDismissBoxValue.Settled -> {}
            }
        }
    }

    SwipeToDismissBox(
        state = dismissState,
        modifier = Modifier.pointerInput(Unit) {
            awaitPointerEventScope {
                while (true) {
                    val event = awaitPointerEvent(PointerEventPass.Initial)
                    // 只要有任何一根手指按在屏幕上，就是 true，松手就是 false
                    isPressed = event.changes.any { it.pressed }
                }
            }
        },
        backgroundContent = {
            val color by animateColorAsState(
                targetValue = when (dismissState.targetValue) {
                    SwipeToDismissBoxValue.StartToEnd -> MaterialTheme.colorScheme.primaryContainer
                    SwipeToDismissBoxValue.EndToStart -> MaterialTheme.colorScheme.errorContainer
                    SwipeToDismissBoxValue.Settled -> Color.Transparent
                },
                label = "dismiss_color",
            )

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(color, RoundedCornerShape(12.dp))
                    .padding(horizontal = 20.dp),
            ) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.CenterStart,
                ) {
                    Icon(
                        imageVector = Icons.Filled.Edit,
                        contentDescription = null,
                    )
                }
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.CenterEnd,
                ) {
                    Icon(
                        imageVector = Icons.Filled.Delete,
                        contentDescription = null,
                    )
                }
            }
        },
    ) {
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surfaceVariant,
            ),
            onClick = onClick,
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = draft.content.take(200),
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 6,
                    overflow = TextOverflow.Ellipsis,
                    color = MaterialTheme.colorScheme.onSurface,
                )

                Spacer(modifier = Modifier.height(8.dp))

                val dateFormat = SimpleDateFormat("yyyy年MM月dd日 HH:mm:ss", Locale.getDefault())
                Text(
                    text = "编辑于${dateFormat.format(Date(draft.savedAt))}",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                val displayTags = NoteColorUtil.filterDisplayTags(draft.tags)
                if (displayTags.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = displayTags.joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}
