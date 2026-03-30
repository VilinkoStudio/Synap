package com.fuwaki.synap.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.FilterList // 新增筛选图标
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet // 新增底部弹窗
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.fuwaki.synap.ui.components.NoteCardItem
import com.fuwaki.synap.ui.model.Note
import com.fuwaki.synap.ui.viewmodel.HomeUiState
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    uiState: HomeUiState,
    onOpenSettings: () -> Unit,
    onComposeNote: () -> Unit,
    onOpenNote: (String) -> Unit,
    onReplyToNote: (String, String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onOpenSearch: () -> Unit,
    onLoadMore: () -> Unit,
    onRefresh: () -> Unit,
) {
    val gridState = rememberLazyStaggeredGridState()

    // --- 新增：控制筛选菜单状态和当前选中的过滤条件 ---
    var showFilterSheet by remember { mutableStateOf(false) }
    var currentFilter by rememberSaveable { mutableStateOf("全部") } // 记住用户的选择

    val isAtTop by remember {
        derivedStateOf {
            gridState.firstVisibleItemIndex == 0
        }
    }

    val shouldLoadMore by remember(uiState.notes, uiState.hasMore, uiState.isLoading) {
        derivedStateOf {
            val lastVisible = gridState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
            uiState.hasMore &&
                    !uiState.isLoading &&
                    uiState.notes.isNotEmpty() &&
                    lastVisible >= uiState.notes.lastIndex - 4
        }
    }

    // --- 修改：在这里根据 currentFilter 状态进一步过滤笔记 ---
    val displayNotes = remember(uiState.notes, currentFilter) {
        val sorted = uiState.notes.sortedBy { it.isDeleted } // 依旧保持正常在前的排序
        when (currentFilter) {
            "正常" -> sorted.filter { !it.isDeleted }
            "已删除" -> sorted.filter { it.isDeleted }
            else -> sorted // "全部"
        }
    }

    LaunchedEffect(gridState, uiState.hasMore, uiState.isLoading, uiState.isSearchMode) {
        snapshotFlow { shouldLoadMore }
            .map { it && !uiState.isSearchMode }
            .distinctUntilChanged()
            .collectLatest { ready ->
                if (ready) {
                    onLoadMore()
                }
            }
    }

    Scaffold(
        topBar = {
            Column {
                TopAppBar(
                    title = {
                        Surface(
                            color = MaterialTheme.colorScheme.tertiaryContainer,
                            shape = MaterialTheme.shapes.small,
                        ) {
                            Text(
                                text = "Synap",
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                                style = MaterialTheme.typography.titleMedium,
                            )
                        }
                    },
                    actions = {
                        AnimatedVisibility(
                            visible = !isAtTop,
                            enter = fadeIn(),
                            exit = fadeOut()
                        ) {
                            IconButton(onClick = onOpenSearch) {
                                Icon(Icons.Filled.Search, contentDescription = "搜索")
                            }
                        }

                        // --- 新增：筛选图标（在搜索和设置之间）---
                        IconButton(onClick = { showFilterSheet = true }) {
                            Icon(Icons.Filled.FilterList, contentDescription = "筛选")
                        }

                        IconButton(onClick = onOpenSettings) {
                            Icon(Icons.Filled.Settings, contentDescription = "设置")
                        }
                    },
                )

                AnimatedVisibility(
                    visible = isAtTop,
                    enter = expandVertically() + fadeIn(),
                    exit = shrinkVertically() + fadeOut()
                ) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 8.dp)
                    ) {
                        Surface(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(MaterialTheme.shapes.extraLarge)
                                .clickable { onOpenSearch() },
                            color = MaterialTheme.colorScheme.surfaceVariant,
                            shape = MaterialTheme.shapes.extraLarge
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(
                                    imageVector = Icons.Filled.Search,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                                Spacer(modifier = Modifier.width(12.dp))
                                Text(
                                    text = "搜索笔记、标签、片段...",
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    style = MaterialTheme.typography.bodyLarge
                                )
                            }
                        }
                    }
                }
            }
        },
        floatingActionButton = {
            FloatingActionButton(onClick = onComposeNote) {
                Icon(Icons.Filled.Add, contentDescription = "创建笔记")
            }
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            when {
                uiState.isLoading && uiState.notes.isEmpty() -> {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(top = 8.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        CircularProgressIndicator()
                    }
                }

                uiState.errorMessage != null && uiState.notes.isEmpty() -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                    ) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text(
                                text = uiState.errorMessage,
                                color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            TextButton(
                                onClick = onRefresh,
                                modifier = Modifier.padding(top = 12.dp),
                            ) {
                                Text("重试")
                            }
                        }
                    }
                }

                displayNotes.isEmpty() -> { // 修改：这里根据过滤后的列表判断是否为空
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = "这里空空如也",
                            style = MaterialTheme.typography.bodyLarge,
                        )
                    }
                }

                else -> {
                    LazyVerticalStaggeredGrid(
                        columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
                        state = gridState,
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(16.dp),
                        verticalItemSpacing = 16.dp,
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        itemsIndexed(displayNotes, key = { _, note -> note.id }) { index, note ->
                            NoteCardItem(
                                note = note,
                                onClick = { onOpenNote(note.id) },
                                onToggleDeleted = { onToggleDeleted(note) },
                                onReply = { onReplyToNote(note.id, note.content) },
                                animationDelayMillis = (index.coerceAtMost(6)) * 45,
                            )
                        }
                    }
                }
            }
        }
    }

    // --- 新增：底部筛选弹窗 ModalBottomSheet ---
    if (showFilterSheet) {
        ModalBottomSheet(
            onDismissRequest = { showFilterSheet = false }
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 24.dp)
                    .padding(bottom = 48.dp) // 给底部留一点安全距离
            ) {
                Text(
                    text = "筛选",
                    style = MaterialTheme.typography.titleLarge,
                    modifier = Modifier.padding(bottom = 16.dp)
                )

                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    FilterChip(
                        selected = currentFilter == "全部",
                        onClick = {
                            currentFilter = "全部"
                            showFilterSheet = false // 点击后自动收起弹窗，体验更顺畅
                        },
                        label = { Text("全部") }
                    )
                    FilterChip(
                        selected = currentFilter == "正常",
                        onClick = {
                            currentFilter = "正常"
                            showFilterSheet = false
                        },
                        label = { Text("正常") }
                    )
                    FilterChip(
                        selected = currentFilter == "已删除",
                        onClick = {
                            currentFilter = "已删除"
                            showFilterSheet = false
                        },
                        label = { Text("已删除") }
                    )
                }
            }
        }
    }
}