package com.synap.app.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.HomeFilterBar
import com.synap.app.ui.components.HomeNoteFeed
import com.synap.app.ui.components.HomeSessionFeed
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.TimelineSessionGroup
import com.synap.app.ui.theme.MyApplicationTheme
import com.synap.app.ui.viewmodel.HomeUiState
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlin.math.PI
import kotlin.math.sin
import androidx.compose.runtime.saveable.rememberSaveable

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
    onOpenTrash: () -> Unit,
    onLoadMore: () -> Unit,
    onRefresh: () -> Unit,
    onSetFilterPanelOpen: (Boolean) -> Unit,
    onToggleTagFilter: (String) -> Unit,
    onToggleUntaggedFilter: () -> Unit,
    onToggleAllTags: () -> Unit,
) {
    val noteGridState = rememberLazyStaggeredGridState()
    val sessionGridState = rememberLazyStaggeredGridState()
    val scope = rememberCoroutineScope()

    var deletedNoteToUndo by remember { mutableStateOf<Note?>(null) }
    var undoProgress by remember { mutableFloatStateOf(1f) }
    var timeLeftSeconds by remember { mutableIntStateOf(3) }

    var pendingDeleteNoteIds by remember { mutableStateOf(setOf<String>()) }

    // --- 多选模式状态 ---
    var isSelectionMode by rememberSaveable { mutableStateOf(false) }
    var selectedNoteIds by rememberSaveable { mutableStateOf(setOf<String>()) }

    // 物理返回键拦截（处于多选模式时按返回键取消选择）
    BackHandler(enabled = isSelectionMode) {
        isSelectionMode = false
        selectedNoteIds = emptySet()
    }

    fun toggleSelection(noteId: String) {
        selectedNoteIds = if (selectedNoteIds.contains(noteId)) {
            selectedNoteIds - noteId
        } else {
            selectedNoteIds + noteId
        }
        // 如果最后取消选中了所有内容，自动退出多选模式
        if (selectedNoteIds.isEmpty()) {
            isSelectionMode = false
        }
    }

    fun finalizePendingDelete(note: Note) {
        if (note.id in pendingDeleteNoteIds) {
            pendingDeleteNoteIds = pendingDeleteNoteIds - note.id
            onToggleDeleted(note)
        }
    }

    fun showUndoForDeletedNote(note: Note) {
        deletedNoteToUndo?.takeIf { it.id != note.id }?.let(::finalizePendingDelete)
        pendingDeleteNoteIds = pendingDeleteNoteIds + note.id
        deletedNoteToUndo = note
        undoProgress = 1f
        timeLeftSeconds = 3
    }

    val fabDodgeOffset by animateDpAsState(
        targetValue = if (deletedNoteToUndo != null) (-96).dp else 0.dp,
        label = "fab_dodge_animation"
    )

    LaunchedEffect(deletedNoteToUndo) {
        val note = deletedNoteToUndo
        if (note != null) {
            undoProgress = 1f
            timeLeftSeconds = 3
            var timeLeft = 3000L
            val interval = 16L
            while (timeLeft > 0) {
                delay(interval)
                timeLeft -= interval
                undoProgress = timeLeft.toFloat() / 3000f
                timeLeftSeconds = kotlin.math.ceil(timeLeft / 1000f).toInt()
            }
            finalizePendingDelete(note)
            deletedNoteToUndo = null
        }
    }

    val isShowingSessionFeed = uiState.showSessionFeed && !uiState.isSearchMode

    val isScrolledDown by remember(noteGridState, sessionGridState, isShowingSessionFeed) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                sessionGridState.firstVisibleItemIndex > 0 || sessionGridState.firstVisibleItemScrollOffset > 100
            } else {
                noteGridState.firstVisibleItemIndex > 0 || noteGridState.firstVisibleItemScrollOffset > 100
            }
        }
    }

    val isAtTop by remember(noteGridState, sessionGridState, isShowingSessionFeed) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                sessionGridState.firstVisibleItemIndex == 0 && sessionGridState.firstVisibleItemScrollOffset <= 10
            } else {
                noteGridState.firstVisibleItemIndex == 0 && noteGridState.firstVisibleItemScrollOffset <= 10
            }
        }
    }

    var isTagsExpanded by rememberSaveable { mutableStateOf(false) }

    val displayNotes = remember(uiState.notes, pendingDeleteNoteIds) {
        uiState.notes
            .distinctBy { it.id }
            .filter { it.id !in pendingDeleteNoteIds }
    }

    val displaySessionGroups = remember(uiState.sessionGroups, pendingDeleteNoteIds) {
        uiState.sessionGroups
            .mapNotNull { session ->
                val notes = session.notes
                    .distinctBy { it.id }
                    .filter { it.id !in pendingDeleteNoteIds }
                if (notes.isEmpty()) {
                    null
                } else {
                    session.copy(
                        noteCount = notes.size,
                        notes = notes,
                    )
                }
            }
    }

    // 批量删除函数
    fun deleteSelectedNotes() {
        val notesToDelete = displayNotes.filter { it.id in selectedNoteIds } +
                displaySessionGroups.flatMap { it.notes }.filter { it.id in selectedNoteIds }

        notesToDelete.distinctBy { it.id }.forEach { note ->
            onToggleDeleted(note)
        }

        // 恢复正常模式
        isSelectionMode = false
        selectedNoteIds = emptySet()
    }

    val shouldLoadMore by remember(
        noteGridState,
        sessionGridState,
        displayNotes,
        displaySessionGroups,
        uiState.hasMore,
        uiState.isLoading,
        isShowingSessionFeed,
    ) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                val lastVisible = sessionGridState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
                val triggerIndex = (displaySessionGroups.lastIndex - 1).coerceAtLeast(0)
                uiState.hasMore &&
                        !uiState.isLoading &&
                        displaySessionGroups.isNotEmpty() &&
                        lastVisible >= triggerIndex
            } else {
                val lastVisible = noteGridState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
                uiState.hasMore &&
                        !uiState.isLoading &&
                        displayNotes.isNotEmpty() &&
                        lastVisible >= displayNotes.lastIndex - 4
            }
        }
    }

    val isActiveFeedEmpty = if (isShowingSessionFeed) {
        displaySessionGroups.isEmpty()
    } else {
        displayNotes.isEmpty()
    }

    LaunchedEffect(
        noteGridState,
        sessionGridState,
        uiState.hasMore,
        uiState.isLoading,
        uiState.isSearchMode,
        isShowingSessionFeed,
    ) {
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
                if (isSelectionMode) {
                    TopAppBar(
                        title = { Text(stringResource(R.string.selected) + " ${selectedNoteIds.size} ") },
                        navigationIcon = {
                            IconButton(onClick = {
                                isSelectionMode = false
                                selectedNoteIds = emptySet()
                            }) {
                                Icon(Icons.Filled.Close, contentDescription = stringResource(R.string.clear))
                            }
                        }
                    )
                } else {
                    TopAppBar(
                        title = {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Text(
                                    text = "Synap",
                                    style = MaterialTheme.typography.titleLarge,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.primary
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
                                    Icon(Icons.Filled.Search, contentDescription = stringResource(R.string.content_desc_search))
                                }
                            }

                            IconButton(onClick = onOpenTrash) {
                                Icon(Icons.Filled.DeleteSweep, contentDescription = stringResource(R.string.trash_title))
                            }

                            IconButton(onClick = onOpenSettings) {
                                Icon(Icons.Filled.Settings, contentDescription = stringResource(R.string.content_desc_settings))
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
                                        text = stringResource(R.string.home_search_title),
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        style = MaterialTheme.typography.bodyLarge
                                    )
                                }
                            }
                        }
                    }
                }
            }
        },
        floatingActionButton = {
            // 只保留原有的常规悬浮按钮在这个插槽里
            AnimatedVisibility(
                visible = !isSelectionMode,
                enter = fadeIn() + slideInVertically(initialOffsetY = { it }),
                exit = fadeOut() + slideOutVertically(targetOffsetY = { it })
            ) {
                Column(
                    horizontalAlignment = Alignment.End,
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                    modifier = Modifier.offset(y = fabDodgeOffset)
                ) {
                    AnimatedVisibility(
                        visible = isScrolledDown,
                        enter = fadeIn(),
                        exit = fadeOut()
                    ) {
                        ExtendedFloatingActionButton(
                            onClick = {
                                scope.launch {
                                    if (isShowingSessionFeed) {
                                        sessionGridState.animateScrollToItem(0)
                                    } else {
                                        noteGridState.animateScrollToItem(0)
                                    }
                                }
                            },
                            icon = { Icon(Icons.Filled.ArrowUpward, contentDescription = null) },
                            text = { Text(text = stringResource(R.string.backtop)) },
                            containerColor = MaterialTheme.colorScheme.secondaryContainer,
                            contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                        )
                    }

                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(16.dp)
                    ) {
                        AnimatedVisibility(
                            visible = !uiState.isSearchMode,
                            enter = fadeIn() + slideInHorizontally { it / 2 },
                            exit = fadeOut() + slideOutHorizontally { it / 2 }
                        ) {
                            val isFeed = !uiState.showSessionFeed
                            Surface(
                                shape = RoundedCornerShape(12.dp),
                                shadowElevation = 6.dp,
                                color = MaterialTheme.colorScheme.surfaceVariant,
                                modifier = Modifier.height(56.dp)
                            ) {
                                Row {
                                    Surface(
                                        onClick = {
                                            onSetFilterPanelOpen(true)
                                            scope.launch { noteGridState.animateScrollToItem(0) }
                                        },
                                        shape = RoundedCornerShape(topStart = 12.dp, bottomStart = 12.dp),
                                        color = if (isFeed) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                                        modifier = Modifier.fillMaxHeight()
                                    ) {
                                        Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.Center,
                                            modifier = Modifier.padding(horizontal = 16.dp)
                                        ) {
                                            AnimatedVisibility(visible = isFeed) {
                                                Row {
                                                    Icon(
                                                        imageVector = Icons.Filled.Check,
                                                        contentDescription = null,
                                                        modifier = Modifier.size(18.dp),
                                                        tint = MaterialTheme.colorScheme.onPrimaryContainer
                                                    )
                                                    Spacer(modifier = Modifier.width(6.dp))
                                                }
                                            }
                                            Text(
                                                text = stringResource(R.string.home_feed_waterfall),
                                                maxLines = 1,
                                                softWrap = false,
                                                style = MaterialTheme.typography.titleSmall,
                                                color = if (isFeed) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant
                                            )
                                        }
                                    }

                                    Surface(
                                        onClick = {
                                            onSetFilterPanelOpen(false)
                                            scope.launch { sessionGridState.animateScrollToItem(0) }
                                        },
                                        shape = RoundedCornerShape(topEnd = 12.dp, bottomEnd = 12.dp),
                                        color = if (!isFeed) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                                        modifier = Modifier.fillMaxHeight()
                                    ) {
                                        Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.Center,
                                            modifier = Modifier.padding(horizontal = 16.dp)
                                        ) {
                                            AnimatedVisibility(visible = !isFeed) {
                                                Row {
                                                    Icon(
                                                        imageVector = Icons.Filled.Check,
                                                        contentDescription = null,
                                                        modifier = Modifier.size(18.dp),
                                                        tint = MaterialTheme.colorScheme.onPrimaryContainer
                                                    )
                                                    Spacer(modifier = Modifier.width(6.dp))
                                                }
                                            }
                                            Text(
                                                text = stringResource(R.string.home_feed_timeline),
                                                maxLines = 1,
                                                softWrap = false,
                                                style = MaterialTheme.typography.titleSmall,
                                                color = if (!isFeed) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        FloatingActionButton(
                            onClick = onComposeNote,
                            modifier = Modifier.size(56.dp)
                        ) {
                            Icon(Icons.Filled.Add, contentDescription = stringResource(R.string.home_creatnote))
                        }
                    }
                }
            }
        },
        bottomBar = {}
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            Column(
                modifier = Modifier.fillMaxSize(),
            ) {
                AnimatedVisibility(
                    visible = !isShowingSessionFeed && !uiState.isSearchMode && !isSelectionMode,
                    enter = expandVertically() + fadeIn(),
                    exit = shrinkVertically() + fadeOut()
                ) {
                    LazyRow(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        item {
                            val isAllSelected = uiState.unselectedTags.isEmpty() && !uiState.isUntaggedUnselected
                            FilterChip(
                                selected = isAllSelected,
                                onClick = onToggleAllTags,
                                label = { Text(stringResource(R.string.home_filter_all)) }
                            )
                        }
                        item {
                            FilterChip(
                                selected = !uiState.isUntaggedUnselected,
                                onClick = onToggleUntaggedFilter,
                                label = { Text(stringResource(R.string.home_filter_untagged)) }
                            )
                        }
                        items(uiState.availableTags) { tag ->
                            FilterChip(
                                selected = tag !in uiState.unselectedTags,
                                onClick = { onToggleTagFilter(tag) },
                                label = { Text(tag) }
                            )
                        }
                    }
                }

                when {
                    uiState.isLoading && isActiveFeedEmpty -> {
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxWidth()
                                .padding(top = 8.dp),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator()
                        }
                    }

                    uiState.errorMessage != null && isActiveFeedEmpty -> {
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxWidth()
                                .verticalScroll(rememberScrollState()),
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
                                    Text(stringResource(R.string.retry))
                                }
                            }
                        }
                    }

                    isActiveFeedEmpty -> {
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxWidth()
                                .verticalScroll(rememberScrollState()),
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = stringResource(R.string.home_empty),
                                style = MaterialTheme.typography.bodyLarge,
                            )
                        }
                    }

                    else -> {
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxWidth(),
                        ) {
                            if (isShowingSessionFeed) {
                                HomeSessionFeed(
                                    sessions = displaySessionGroups,
                                    state = sessionGridState,
                                    isSelectionMode = isSelectionMode,
                                    selectedNoteIds = selectedNoteIds,
                                    onToggleSelection = ::toggleSelection,
                                    onEnterSelectionMode = { id ->
                                        isSelectionMode = true
                                        toggleSelection(id)
                                    },
                                    onOpenNote = onOpenNote,
                                    onToggleDeleted = { note ->
                                        if (!note.isDeleted) {
                                            showUndoForDeletedNote(note)
                                        } else {
                                            onToggleDeleted(note)
                                        }
                                    },
                                    onReplyToNote = onReplyToNote,
                                )
                            } else {
                                HomeNoteFeed(
                                    notes = displayNotes,
                                    state = noteGridState,
                                    isSelectionMode = isSelectionMode,
                                    selectedNoteIds = selectedNoteIds,
                                    onToggleSelection = ::toggleSelection,
                                    onEnterSelectionMode = { id ->
                                        isSelectionMode = true
                                        toggleSelection(id)
                                    },
                                    onOpenNote = onOpenNote,
                                    onToggleDeleted = { note ->
                                        if (!note.isDeleted) {
                                            showUndoForDeletedNote(note)
                                        } else {
                                            onToggleDeleted(note)
                                        }
                                    },
                                    onReplyToNote = onReplyToNote,
                                )
                            }
                        }
                    }
                }
            }

            AnimatedVisibility(
                visible = deletedNoteToUndo != null,
                enter = slideInVertically(initialOffsetY = { it }) + fadeIn(),
                exit = slideOutVertically(targetOffsetY = { it }) + fadeOut(),
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = 32.dp, start = 16.dp, end = 16.dp)
                    .fillMaxWidth()
            ) {
                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = MaterialTheme.colorScheme.secondaryContainer,
                    contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                    shadowElevation = 6.dp
                ) {
                    Column {
                        Row(
                            modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp).fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Text(stringResource(R.string.home_deleted_note), style = MaterialTheme.typography.bodyMedium)
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(
                                    text = "${timeLeftSeconds}s",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.primary,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                            Text(
                                text = stringResource(R.string.home_undo_delete),
                                color = MaterialTheme.colorScheme.primary,
                                style = MaterialTheme.typography.labelLarge,
                                modifier = Modifier
                                    .clip(RoundedCornerShape(4.dp))
                                    .clickable {
                                        deletedNoteToUndo?.let { note ->
                                            pendingDeleteNoteIds = pendingDeleteNoteIds - note.id
                                        }
                                        deletedNoteToUndo = null
                                    }
                                    .padding(8.dp)
                            )
                        }

                        WavyProgressIndicator(
                            progress = undoProgress,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(12.dp)
                                .padding(bottom = 4.dp),
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                }
            }

            // --- 将悬浮多选工具栏放在 Box 层，脱离 FAB 限制实现真正的绝对居中，增加 16dp 底边距 ---
            AnimatedVisibility(
                visible = isSelectionMode,
                enter = fadeIn() + slideInVertically(initialOffsetY = { it }),
                exit = fadeOut() + slideOutVertically(targetOffsetY = { it }),
                modifier = Modifier.align(Alignment.BottomCenter) // 完美的居中对齐
            ) {
                Surface(
                    shape = RoundedCornerShape(percent = 50), // 完全圆角，胶囊状
                    shadowElevation = 8.dp, // 提升悬浮层级
                    color = MaterialTheme.colorScheme.primaryContainer, // Vibrant 背景色
                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer, // 对应的图标颜色
                    modifier = Modifier.padding(bottom = 16.dp) // 距离底部留出 16dp 间距
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp), // 内部元件的上下左右留白
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // 分享按钮
                        IconButton(
                            onClick = { /* TODO: 预留分享功能 */ },
                            enabled = selectedNoteIds.isNotEmpty()
                        ) {
                            Icon(
                                Icons.Filled.Share,
                                contentDescription = "Share",
                                tint = if (selectedNoteIds.isNotEmpty()) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.38f)
                            )
                        }

                        // 删除按钮
                        IconButton(
                            onClick = { deleteSelectedNotes() },
                            enabled = selectedNoteIds.isNotEmpty()
                        ) {
                            Icon(
                                Icons.Filled.Delete,
                                contentDescription = stringResource(R.string.delete),
                                tint = if (selectedNoteIds.isNotEmpty()) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.38f)
                            )
                        }
                    }
                }
            }
        }
    }

}

@Composable
fun WavyProgressIndicator(
    progress: Float,
    modifier: Modifier = Modifier,
    color: Color = MaterialTheme.colorScheme.primary,
) {
    Canvas(modifier = modifier) {
        val path = Path()
        val width = size.width
        val height = size.height
        val midY = height / 2f

        val strokeWidthPx = 4.dp.toPx()
        val amplitude = (height - strokeWidthPx) / 2f
        val waveLength = 20.dp.toPx()

        val endX = width * progress

        if (endX > 0) {
            path.moveTo(0f, midY)
            var x = 0f
            while (x <= endX) {
                val y = midY + sin(x * (2 * PI / waveLength)).toFloat() * amplitude
                path.lineTo(x, y)
                x += 2f
            }
            drawPath(
                path = path,
                color = color,
                style = Stroke(
                    width = strokeWidthPx,
                    cap = StrokeCap.Round,
                    join = StrokeJoin.Round
                )
            )
        }
    }
}