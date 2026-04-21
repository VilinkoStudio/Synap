package com.synap.app.ui.screens

import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
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
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.VerticalAlignTop
import androidx.compose.material.icons.filled.ViewAgenda
import androidx.compose.material.icons.filled.ViewStream
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingToolbarDefaults
import androidx.compose.material3.HorizontalFloatingToolbar
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberModalBottomSheetState
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
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.ShareExportSheet
import com.synap.app.ui.components.HomeNoteFeed
import com.synap.app.ui.components.HomeSessionFeed
import com.synap.app.ui.model.Note
import com.synap.app.ui.viewmodel.HomeUiState
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlin.math.PI
import kotlin.math.sin
import androidx.compose.runtime.saveable.rememberSaveable

@OptIn(ExperimentalMaterial3Api::class, ExperimentalSharedTransitionApi::class, ExperimentalMaterial3ExpressiveApi::class)
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
    onExportShare: suspend (List<String>) -> ByteArray,
    sharedTransitionScope: SharedTransitionScope? = null,
    animatedVisibilityScope: AnimatedVisibilityScope? = null,
) {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    val prefs = remember { context.getSharedPreferences("synap_prefs", Context.MODE_PRIVATE) }

    val navVisibilityScope = animatedVisibilityScope

    val noteGridState = rememberLazyStaggeredGridState()
    val sessionGridState = rememberLazyStaggeredGridState()
    val scope = rememberCoroutineScope()

    val isFeed = !uiState.showSessionFeed

    LaunchedEffect(Unit) {
        val savedMode = prefs.getBoolean("is_waterfall_mode", true)
        onSetFilterPanelOpen(savedMode)
        onRefresh()
    }

    fun switchFeedMode(waterfall: Boolean) {
        prefs.edit().putBoolean("is_waterfall_mode", waterfall).apply()
        onSetFilterPanelOpen(waterfall)
        if (waterfall) {
            scope.launch { noteGridState.animateScrollToItem(0) }
        } else {
            scope.launch { sessionGridState.animateScrollToItem(0) }
        }
    }

    var deletedNoteToUndo by remember { mutableStateOf<Note?>(null) }
    var undoProgress by remember { mutableFloatStateOf(1f) }
    var timeLeftSeconds by remember { mutableIntStateOf(3) }

    var pendingDeleteNoteIds by remember { mutableStateOf(setOf<String>()) }

    var isSelectionMode by rememberSaveable { mutableStateOf(false) }
    var selectedNoteIds by rememberSaveable { mutableStateOf(setOf<String>()) }

    var showFeedMenu by remember { mutableStateOf(false) }
    var showMultiDeleteDialog by remember { mutableStateOf(false) }
    var noteToCopy by remember { mutableStateOf<Note?>(null) }

    var showShareBottomSheet by remember { mutableStateOf(false) }

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

    fun deleteSelectedNotes() {
        val notesToDelete = displayNotes.filter { it.id in selectedNoteIds } +
                displaySessionGroups.flatMap { it.notes }.filter { it.id in selectedNoteIds }

        notesToDelete.distinctBy { it.id }.forEach { note ->
            onToggleDeleted(note)
        }

        isSelectionMode = false
        selectedNoteIds = emptySet()
    }

    val shouldLoadMore by remember(
        noteGridState,
        displayNotes,
        uiState.hasMore,
        uiState.isLoading,
        isShowingSessionFeed,
    ) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                false
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

    if (showMultiDeleteDialog) {
        AlertDialog(
            onDismissRequest = { showMultiDeleteDialog = false },
            title = { Text("确认删除") },
            text = { Text("是否确认删除选择的 ${selectedNoteIds.size} 条笔记？") },
            confirmButton = {
                TextButton(
                    onClick = {
                        showMultiDeleteDialog = false
                        deleteSelectedNotes()
                    }
                ) {
                    Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showMultiDeleteDialog = false }) {
                    Text(stringResource(R.string.cancel))
                }
            }
        )
    }

    noteToCopy?.let { note ->
        AlertDialog(
            onDismissRequest = { noteToCopy = null },
            title = { Text("复制笔记") },
            text = { Text("该笔记包含 Markdown 格式，请选择您要复制的文本格式：") },
            confirmButton = {
                TextButton(onClick = {
                    clipboardManager.setText(AnnotatedString(note.content))
                    Toast.makeText(context, "已复制 Markdown", Toast.LENGTH_SHORT).show()
                    noteToCopy = null
                    isSelectionMode = false
                    selectedNoteIds = emptySet()
                }) {
                    Text("Markdown")
                }
            },
            dismissButton = {
                TextButton(onClick = {
                    val plainText = stripMarkdown(note.content)
                    clipboardManager.setText(AnnotatedString(plainText))
                    Toast.makeText(context, "已复制纯文本", Toast.LENGTH_SHORT).show()
                    noteToCopy = null
                    isSelectionMode = false
                    selectedNoteIds = emptySet()
                }) {
                    Text("纯文本")
                }
            }
        )
    }

    if (showShareBottomSheet && selectedNoteIds.isNotEmpty()) {
        ShareExportSheet(
            noteIds = selectedNoteIds.toList(),
            onDismiss = { showShareBottomSheet = false },
            exportShare = onExportShare,
        )
    }

    Scaffold(
        topBar = {
            Column {
                if (isSelectionMode) {
                    TopAppBar(
                        title = {
                            Text(
                                text = "${stringResource(R.string.selected)} ${selectedNoteIds.size}",
                                style = MaterialTheme.typography.titleLarge,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                        },
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
                            // ========== 修改：将搜索按钮放在扫码按钮前面 ==========
                            AnimatedVisibility(
                                visible = !isAtTop,
                                enter = fadeIn(),
                                exit = fadeOut()
                            ) {
                                IconButton(onClick = onOpenSearch) {
                                    Icon(Icons.Filled.Search, contentDescription = stringResource(R.string.content_desc_search))
                                }
                            }

                            // 扫码按钮
                            IconButton(
                                onClick = {
                                    val scannerPackages = listOf(
                                        "com.xiaomi.scanner",
                                        "com.huawei.scanner",
                                        "com.huawei.hms.image.vision",
                                        "com.coloros.ocrscanner",
                                        "com.vivo.scan",
                                        "com.bbk.vision",
                                        "com.hihonor.vision",
                                        "com.meizu.media.camera",
                                        "com.samsung.android.visionintelligence",
                                        "com.nubia.vision",
                                        "com.google.ar.lens"
                                    )
                                    var opened = false
                                    for (pkg in scannerPackages) {
                                        try {
                                            val intent = context.packageManager.getLaunchIntentForPackage(pkg)
                                            if (intent != null) {
                                                intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                                context.startActivity(intent)
                                                opened = true
                                                break
                                            }
                                        } catch (e: Exception) {
                                        }
                                    }
                                    if (!opened) {
                                        Toast.makeText(context, "未找到系统扫一扫应用", Toast.LENGTH_SHORT).show()
                                    }
                                }
                            ) {
                                Icon(Icons.Filled.QrCodeScanner, contentDescription = "扫一扫")
                            }

                            Box {
                                IconButton(onClick = { showFeedMenu = true }) {
                                    Icon(
                                        imageVector = if (isFeed) Icons.Filled.ViewStream else Icons.Filled.ViewAgenda,
                                        contentDescription = "Switch Feed View"
                                    )
                                }
                                DropdownMenu(
                                    expanded = showFeedMenu,
                                    onDismissRequest = { showFeedMenu = false }
                                ) {
                                    DropdownMenuItem(
                                        text = { Text(stringResource(R.string.home_feed_waterfall)) },
                                        leadingIcon = {
                                            Icon(Icons.Filled.ViewStream, contentDescription = null)
                                        },
                                        trailingIcon = {
                                            if (isFeed) Icon(Icons.Filled.Check, null, modifier = Modifier.size(20.dp), tint = MaterialTheme.colorScheme.primary)
                                        },
                                        onClick = {
                                            switchFeedMode(true)
                                            showFeedMenu = false
                                        }
                                    )
                                    DropdownMenuItem(
                                        text = { Text(stringResource(R.string.home_feed_timeline)) },
                                        leadingIcon = {
                                            Icon(Icons.Filled.ViewAgenda, contentDescription = null)
                                        },
                                        trailingIcon = {
                                            if (!isFeed) Icon(Icons.Filled.Check, null, modifier = Modifier.size(20.dp), tint = MaterialTheme.colorScheme.primary)
                                        },
                                        onClick = {
                                            switchFeedMode(false)
                                            showFeedMenu = false
                                        }
                                    )
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
                                    .let {
                                        if (sharedTransitionScope != null && navVisibilityScope != null) {
                                            with(sharedTransitionScope) {
                                                it.sharedBounds(
                                                    sharedContentState = rememberSharedContentState(key = "search_to_fullscreen"),
                                                    animatedVisibilityScope = navVisibilityScope
                                                )
                                            }
                                        } else {
                                            it
                                        }
                                    }
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
            AnimatedVisibility(
                visible = !isSelectionMode,
                enter = fadeIn() + slideInVertically(initialOffsetY = { it }),
                exit = fadeOut() + slideOutVertically(targetOffsetY = { it }),
                modifier = Modifier.offset(y = fabDodgeOffset)
            ) {
                Column(
                    horizontalAlignment = Alignment.End,
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    AnimatedVisibility(
                        visible = isScrolledDown,
                        enter = fadeIn(),
                        exit = fadeOut()
                    ) {
                        FloatingActionButton(
                            onClick = {
                                scope.launch {
                                    if (isShowingSessionFeed) {
                                        sessionGridState.animateScrollToItem(0)
                                    } else {
                                        noteGridState.animateScrollToItem(0)
                                    }
                                }
                            },
                            containerColor = MaterialTheme.colorScheme.secondaryContainer,
                            contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                        ) {
                            Icon(Icons.Filled.VerticalAlignTop, contentDescription = stringResource(R.string.backtop))
                        }
                    }

                    FloatingActionButton(
                        onClick = onComposeNote,
                        shape = CircleShape, // ========== 修改：将加号按钮设为圆形 ==========
                        modifier = Modifier
                            .size(72.dp)
                            .let {
                                if (sharedTransitionScope != null && navVisibilityScope != null) {
                                    with(sharedTransitionScope) {
                                        it.sharedBounds(
                                            sharedContentState = rememberSharedContentState(key = "fab_to_new_note"),
                                            animatedVisibilityScope = navVisibilityScope
                                        )
                                    }
                                } else {
                                    it
                                }
                            }
                    ) {
                        Icon(
                            Icons.Filled.Add,
                            contentDescription = stringResource(R.string.home_creatnote),
                            modifier = Modifier.size(36.dp)
                        )
                    }
                }
            }
        },
        bottomBar = {}
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(top = innerPadding.calculateTopPadding())
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
                        val isAllSelected =
                            uiState.unselectedTags.isEmpty() && !uiState.isUntaggedUnselected

                        item {
                            FilterChip(
                                selected = isAllSelected,
                                onClick = onToggleAllTags,
                                label = { Text(stringResource(R.string.home_filter_all)) }
                            )
                        }
                        item {
                            FilterChip(
                                selected = !isAllSelected && !uiState.isUntaggedUnselected,
                                onClick = onToggleUntaggedFilter,
                                label = { Text(stringResource(R.string.home_filter_untagged)) }
                            )
                        }
                        items(uiState.availableTags) { tag ->
                            FilterChip(
                                selected = !isAllSelected && tag !in uiState.unselectedTags,
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
                                    hasMore = uiState.hasMore,
                                    isLoadingMore = uiState.isLoading,
                                    onLoadMore = onLoadMore,
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
                                    bottomInset = innerPadding.calculateBottomPadding()
                                )
                            } else {
                                HomeNoteFeed(
                                    notes = displayNotes,
                                    state = noteGridState,
                                    bottomInset = innerPadding.calculateBottomPadding(),
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
                    .padding(bottom = 16.dp + innerPadding.calculateBottomPadding())
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp)
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

            AnimatedVisibility(
                visible = isSelectionMode,
                enter = fadeIn() + slideInVertically(initialOffsetY = { it }),
                exit = fadeOut() + slideOutVertically(targetOffsetY = { it }),
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = 24.dp + innerPadding.calculateBottomPadding())
                    .offset(y = fabDodgeOffset)
            ) {
                HorizontalFloatingToolbar(
                    expanded = true,
                    colors = FloatingToolbarDefaults.standardFloatingToolbarColors(
                        toolbarContainerColor = MaterialTheme.colorScheme.secondaryContainer,
                        toolbarContentColor = MaterialTheme.colorScheme.onSecondaryContainer
                    )
                ) {
                    val iconTint = MaterialTheme.colorScheme.onSecondaryContainer

                    if (selectedNoteIds.size == 1) {
                        IconButton(
                            onClick = {
                                val noteId = selectedNoteIds.first()
                                val note = displayNotes.find { it.id == noteId }
                                    ?: displaySessionGroups.flatMap { it.notes }.find { it.id == noteId }

                                if (note != null) {
                                    val hasMarkdown = Regex("(\\*\\*\\*|\\*\\*|\\*|~~|<u>|==|^#{1,6} |^> |^-\\s+\\[[ x]\\]\\s|^-\\s|^\\d+\\.\\s)", RegexOption.MULTILINE).containsMatchIn(note.content)
                                    if (hasMarkdown) {
                                        noteToCopy = note
                                    } else {
                                        clipboardManager.setText(AnnotatedString(note.content))
                                        Toast.makeText(context, "已复制", Toast.LENGTH_SHORT).show()
                                        isSelectionMode = false
                                        selectedNoteIds = emptySet()
                                    }
                                }
                            }
                        ) {
                            Icon(
                                Icons.Filled.ContentCopy,
                                contentDescription = "复制",
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }
                    }

                    IconButton(
                        onClick = { showShareBottomSheet = true },
                        enabled = selectedNoteIds.isNotEmpty()
                    ) {
                        Icon(
                            Icons.Filled.Share,
                            contentDescription = "Share",
                            modifier = Modifier.size(24.dp),
                            tint = if (selectedNoteIds.isNotEmpty()) iconTint else iconTint.copy(alpha = 0.38f)
                        )
                    }

                    IconButton(
                        onClick = { showMultiDeleteDialog = true },
                        enabled = selectedNoteIds.isNotEmpty()
                    ) {
                        Icon(
                            Icons.Filled.Delete,
                            contentDescription = stringResource(R.string.delete),
                            modifier = Modifier.size(24.dp),
                            tint = if (selectedNoteIds.isNotEmpty()) iconTint else iconTint.copy(alpha = 0.38f)
                        )
                    }
                }
            }
        }
    }
}

// ==================== 辅助工具函数 ====================
fun stripMarkdown(text: String): String {
    var result = text
    result = result.replace(Regex("^(#{1,6})\\s+", RegexOption.MULTILINE), "")
    result = result.replace(Regex("\\*\\*\\*(.*?)\\*\\*\\*"), "$1")
    result = result.replace(Regex("\\*\\*(.*?)\\*\\*"), "$1")
    result = result.replace(Regex("\\*(.*?)\\*"), "$1")
    result = result.replace(Regex("~~(.*?)~~"), "$1")
    result = result.replace(Regex("==(.*?)=="), "$1")
    result = result.replace(Regex("<u>(.*?)</u>"), "$1")
    result = result.replace(Regex("^>\\s+", RegexOption.MULTILINE), "")
    result = result.replace(Regex("^-\\s+\\[[ x]\\]\\s+", RegexOption.MULTILINE), "")
    result = result.replace(Regex("^-\\s+", RegexOption.MULTILINE), "")
    result = result.replace(Regex("^\\d+\\.\\s+", RegexOption.MULTILINE), "")
    return result
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

// ==================== 二维码生成工具 ====================
fun generateQRCodeBitmapForHome(text: String, size: Int = 512, primaryColor: Int = android.graphics.Color.BLACK, backgroundColor: Int = android.graphics.Color.WHITE): android.graphics.Bitmap? {
    if (text.isEmpty()) return null
    return try {
        val hints = mapOf(
            com.google.zxing.EncodeHintType.CHARACTER_SET to "UTF-8",
            com.google.zxing.EncodeHintType.MARGIN to 1
        )
        val bitMatrix = com.google.zxing.MultiFormatWriter().encode(
            text,
            com.google.zxing.BarcodeFormat.QR_CODE,
            size,
            size,
            hints
        )
        val width = bitMatrix.width
        val height = bitMatrix.height
        val pixels = IntArray(width * height)
        for (y in 0 until height) {
            val offset = y * width
            for (x in 0 until width) {
                pixels[offset + x] = if (bitMatrix[x, y]) primaryColor else backgroundColor
            }
        }
        val bitmap = android.graphics.Bitmap.createBitmap(width, height, android.graphics.Bitmap.Config.ARGB_8888)
        bitmap.setPixels(pixels, 0, width, 0, 0, width, height)
        bitmap
    } catch (e: Exception) {
        e.printStackTrace()
        null
    }
}