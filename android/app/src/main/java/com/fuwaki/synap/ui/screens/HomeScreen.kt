package com.fuwaki.synap.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SearchBar
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
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
    onToggleDeletedFeed: () -> Unit,
    onSearchQueryChange: (String) -> Unit,
    onSubmitSearch: () -> Unit,
    onClearSearch: () -> Unit,
    onLoadMore: () -> Unit,
    onRefresh: () -> Unit,
) {
    val gridState = rememberLazyStaggeredGridState()
    var searchActive by rememberSaveable { mutableStateOf(false) }
    val shouldLoadMore by remember(uiState.notes, uiState.hasMore, uiState.isLoading) {
        derivedStateOf {
            val lastVisible = gridState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
            uiState.hasMore &&
                !uiState.isLoading &&
                uiState.notes.isNotEmpty() &&
                lastVisible >= uiState.notes.lastIndex - 4
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
                        if (searchActive) {
                            TextButton(
                                onClick = {
                                    searchActive = false
                                    if (uiState.query.isBlank()) {
                                        onClearSearch()
                                    }
                                },
                            ) {
                                Text("完成")
                            }
                        } else {
                            IconButton(onClick = { searchActive = true }) {
                                Icon(Icons.Filled.Search, contentDescription = "搜索")
                            }
                            IconButton(onClick = onOpenSettings) {
                                Icon(Icons.Filled.Settings, contentDescription = "设置")
                            }
                        }
                    },
                )

                AnimatedVisibility(
                    visible = searchActive,
                    enter = fadeIn() + expandVertically(),
                    exit = fadeOut() + shrinkVertically(),
                ) {
                    SearchBar(
                        query = uiState.query,
                        onQueryChange = onSearchQueryChange,
                        onSearch = { onSubmitSearch() },
                        active = false,
                        onActiveChange = {},
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(start = 12.dp, end = 12.dp, bottom = 8.dp),
                        placeholder = { Text("搜索笔记、标签、片段") },
                        leadingIcon = {
                            Icon(Icons.Filled.Search, contentDescription = null)
                        },
                        trailingIcon = {
                            if (uiState.query.isNotBlank()) {
                                IconButton(onClick = onClearSearch) {
                                    Icon(Icons.Filled.Clear, contentDescription = "清除")
                                }
                            }
                        },
                    ) {}
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
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                FilterChip(
                    selected = !uiState.showDeleted,
                    onClick = {
                        if (uiState.showDeleted) {
                            onToggleDeletedFeed()
                        }
                    },
                    label = { Text("时间线") },
                    enabled = !uiState.isSearchMode,
                )
                FilterChip(
                    selected = uiState.showDeleted,
                    onClick = {
                        if (!uiState.showDeleted) {
                            onToggleDeletedFeed()
                        }
                    },
                    label = { Text("删除流") },
                    enabled = !uiState.isSearchMode,
                )
                AnimatedContent(
                    targetState = when {
                        uiState.isSearchMode -> "搜索结果"
                        searchActive -> "输入关键词后搜索"
                        uiState.showDeleted -> "查看已删除节点"
                        else -> "按时间浏览最新节点"
                    },
                    label = "home_hint",
                ) { hint ->
                    Text(
                        text = hint,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

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

                uiState.notes.isEmpty() -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = when {
                                uiState.isSearchMode -> "没有匹配的笔记"
                                uiState.showDeleted -> "删除流为空"
                                else -> "还没有笔记"
                            },
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
                        itemsIndexed(uiState.notes, key = { _, note -> note.id }) { index, note ->
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
}
