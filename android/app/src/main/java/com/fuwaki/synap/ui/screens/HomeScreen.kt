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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.FilterList
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.fuwaki.synap.ui.components.NoteCardItem
import com.fuwaki.synap.ui.model.Note
import com.fuwaki.synap.ui.viewmodel.HomeUiState
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

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
    val scope = rememberCoroutineScope()

    var showFilterSheet by remember { mutableStateOf(false) }
    var currentFilter by rememberSaveable { mutableStateOf("全部") }

    val isScrolledDown by remember {
        derivedStateOf {
            gridState.firstVisibleItemIndex > 0 || gridState.firstVisibleItemScrollOffset > 100
        }
    }

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

    val displayNotes = remember(uiState.notes, currentFilter) {
        val sorted = uiState.notes.sortedBy { it.isDeleted }
        when (currentFilter) {
            "正常" -> sorted.filter { !it.isDeleted }
            "已删除" -> sorted.filter { it.isDeleted }
            else -> sorted
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
                        // --- 修改：去掉了这里的 Surface 背景，改成了纯文字 ---
                        Text(
                            text = "Synap",
                            style = MaterialTheme.typography.titleLarge,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.primary
                        )
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
            Column(
                horizontalAlignment = Alignment.End,
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                AnimatedVisibility(
                    visible = isScrolledDown,
                    enter = fadeIn(),
                    exit = fadeOut()
                ) {
                    ExtendedFloatingActionButton(
                        onClick = {
                            scope.launch {
                                gridState.animateScrollToItem(0)
                            }
                        },
                        icon = { Icon(Icons.Filled.ArrowUpward, contentDescription = null) },
                        text = { Text(text = "回到顶部") },
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                    )
                }

                FloatingActionButton(onClick = onComposeNote) {
                    Icon(Icons.Filled.Add, contentDescription = "创建笔记")
                }
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

                displayNotes.isEmpty() -> {
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

    if (showFilterSheet) {
        ModalBottomSheet(
            onDismissRequest = { showFilterSheet = false }
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 24.dp)
                    .padding(bottom = 48.dp)
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
                            showFilterSheet = false
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