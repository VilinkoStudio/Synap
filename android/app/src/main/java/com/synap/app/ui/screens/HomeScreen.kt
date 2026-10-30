package com.synap.app.ui.screens

import android.content.Context
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
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
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.VerticalAlignTop
import androidx.compose.material.icons.filled.Map
import androidx.compose.material.icons.automirrored.filled.MenuOpen
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilterChip
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingToolbarDefaults
import androidx.compose.material3.HorizontalFloatingToolbar
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SheetState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.WideNavigationRail
import androidx.compose.material3.WideNavigationRailDefaults
import androidx.compose.material3.WideNavigationRailItem
import androidx.compose.material3.WideNavigationRailValue
import androidx.compose.material3.rememberWideNavigationRailState
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.ShareExportSheet
import com.synap.app.ui.components.HomeTimelineFeed
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.committedSelectedNoteIds
import com.synap.app.ui.viewmodel.HomeUiState
import com.synap.app.ui.viewmodel.TimelineBrowserUiState
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.math.PI
import kotlin.math.sin
import androidx.compose.runtime.saveable.rememberSaveable
import java.time.LocalDate
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min

@OptIn(ExperimentalMaterial3Api::class, ExperimentalSharedTransitionApi::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun HomeScreen(
    uiState: HomeUiState,
    buildVersion: String = "",
    onOpenSettings: () -> Unit,
    onComposeNote: () -> Unit,
    onOpenNote: (String) -> Unit,
    onOpenDraft: (String) -> Unit,
    onOpenRelatedNote: (String) -> Unit,
    onReplyToNote: (String, String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onOpenSearch: () -> Unit,
    onOpenScanner: () -> Unit,
    onOpenTrash: () -> Unit,
    onLoadMore: () -> Unit,
    onOpenTimelineBrowser: () -> Unit,
    onSelectTimelineDate: (LocalDate) -> Unit,
    onRefresh: () -> Unit,
    onToggleTagFilter: (String) -> Unit,
    onToggleUntaggedFilter: () -> Unit,
    onToggleAllTags: () -> Unit,
    onExportShare: suspend (List<String>) -> ByteArray,
    sharedTransitionScope: SharedTransitionScope? = null,
    animatedVisibilityScope: AnimatedVisibilityScope? = null,
) {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current

    val navVisibilityScope = animatedVisibilityScope

    val noteGridState = rememberLazyStaggeredGridState()
    val scope = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        onRefresh()
    }

    var deletedNoteToUndo by remember { mutableStateOf<Note?>(null) }
    var undoProgress by remember { mutableFloatStateOf(1f) }
    var timeLeftSeconds by remember { mutableIntStateOf(3) }

    var pendingDeleteNoteIds by remember { mutableStateOf(setOf<String>()) }

    var isSelectionMode by rememberSaveable { mutableStateOf(false) }
    var selectedNoteIds by rememberSaveable { mutableStateOf(setOf<String>()) }

    var showMultiDeleteDialog by remember { mutableStateOf(false) }
    var draftToDiscard by remember { mutableStateOf<Note?>(null) }
    var noteToCopy by remember { mutableStateOf<Note?>(null) }

    var showShareBottomSheet by remember { mutableStateOf(false) }
    var showTimelineBrowser by remember { mutableStateOf(false) }
    val timelineBrowserSheetState = rememberModalBottomSheetState(skipPartiallyExpanded = false)

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

    val isScrolledDown by remember(noteGridState) {
        derivedStateOf {
            noteGridState.firstVisibleItemIndex > 0 || noteGridState.firstVisibleItemScrollOffset > 100
        }
    }

    val isAtTop by remember(noteGridState) {
        derivedStateOf {
            noteGridState.firstVisibleItemIndex == 0 && noteGridState.firstVisibleItemScrollOffset <= 10
        }
    }

    val displayNotes = remember(uiState.notes, pendingDeleteNoteIds) {
        uiState.notes
            .distinctBy { it.id }
            .filter { it.id !in pendingDeleteNoteIds }
    }
    val selectedCommittedNoteIds = remember(displayNotes, selectedNoteIds) {
        committedSelectedNoteIds(displayNotes, selectedNoteIds)
    }

    fun deleteSelectedNotes() {
        val notesToDelete = displayNotes.filter { it.id in selectedNoteIds }

        notesToDelete.distinctBy { it.id }.forEach { note ->
            onToggleDeleted(note)
        }

        isSelectionMode = false
        selectedNoteIds = emptySet()
    }

    val isActiveFeedEmpty = displayNotes.isEmpty()

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

    draftToDiscard?.let { draft ->
        AlertDialog(
            onDismissRequest = { draftToDiscard = null },
            title = { Text("删除未保存笔记？") },
            text = { Text("这条未保存笔记不会进入回收站，删除后无法恢复。") },
            confirmButton = {
                TextButton(
                    onClick = {
                        draftToDiscard = null
                        onToggleDeleted(draft)
                    },
                ) {
                    Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { draftToDiscard = null }) {
                    Text(stringResource(R.string.cancel))
                }
            },
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

    if (showShareBottomSheet && selectedCommittedNoteIds.isNotEmpty()) {
        ShareExportSheet(
            noteIds = selectedCommittedNoteIds.toList(),
            onDismiss = { showShareBottomSheet = false },
            exportShare = onExportShare,
        )
    }

    if (showTimelineBrowser) {
        TimelineBrowserSheet(
            uiState = uiState.timelineBrowser,
            onDismiss = { showTimelineBrowser = false },
            onSelectDate = { date ->
                onSelectTimelineDate(date)
                scope.launch {
                    noteGridState.scrollToItem(0)
                }
            },
            sheetState = timelineBrowserSheetState,
        )
    }

    // ==================== 提取公用局部 UI 组件 ====================
    @Composable
    fun SharedSearchBox(modifier: Modifier = Modifier) {
        Surface(
            modifier = modifier
                .fillMaxWidth()
                .let {
                    if (sharedTransitionScope != null && navVisibilityScope != null) {
                        with(sharedTransitionScope) {
                            it.sharedBounds(
                                sharedContentState = rememberSharedContentState(key = "search_to_fullscreen"),
                                animatedVisibilityScope = navVisibilityScope
                            )
                        }
                    } else it
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

    @Composable
    fun SharedTags() {
        val isAllSelected = uiState.unselectedTags.isEmpty() && !uiState.isUntaggedUnselected
        LazyRow(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            item { FilterChip(selected = isAllSelected, onClick = onToggleAllTags, label = { Text(stringResource(R.string.home_filter_all), maxLines = 1, overflow = TextOverflow.Ellipsis) }) }
            item { FilterChip(selected = !isAllSelected && !uiState.isUntaggedUnselected, onClick = onToggleUntaggedFilter, label = { Text(stringResource(R.string.home_filter_untagged), maxLines = 1, overflow = TextOverflow.Ellipsis) }) }
            items(uiState.availableTags) { tag ->
                FilterChip(
                    selected = !isAllSelected && tag !in uiState.unselectedTags,
                    onClick = { onToggleTagFilter(tag) },
                    label = { Text(tag, maxLines = 1, overflow = TextOverflow.Ellipsis) }
                )
            }
        }
    }

    @Composable
    fun SharedFeedArea(bottomInset: Dp) {
        when {
            uiState.isLoading && isActiveFeedEmpty -> {
                Box(
                    modifier = Modifier.fillMaxSize().padding(top = 8.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    CircularProgressIndicator()
                }
            }
            uiState.errorMessage != null && isActiveFeedEmpty -> {
                Box(
                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                    contentAlignment = Alignment.Center,
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text(
                            text = uiState.errorMessage,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        TextButton(onClick = onRefresh, modifier = Modifier.padding(top = 12.dp)) {
                            Text(stringResource(R.string.retry))
                        }
                    }
                }
            }
            isActiveFeedEmpty -> {
                Box(
                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(text = stringResource(R.string.home_empty), style = MaterialTheme.typography.bodyLarge)
                }
            }
            else -> {
                HomeTimelineFeed(
                    notes = displayNotes,
                    showTimeGroups = uiState.showTimeGroups,
                    dualColumnCards = uiState.dualColumnCards,
                    state = noteGridState,
                    bottomInset = bottomInset,
                    hasMore = uiState.hasMore && !uiState.isSearchMode,
                    isLoadingMore = uiState.isLoading,
                    onLoadMore = onLoadMore,
                    isSelectionMode = isSelectionMode,
                    selectedNoteIds = selectedNoteIds,
                    onToggleSelection = ::toggleSelection,
                    onEnterSelectionMode = { id -> isSelectionMode = true; toggleSelection(id) },
                    onOpenNote = onOpenNote,
                    onOpenDraft = onOpenDraft,
                    onOpenRelatedNote = onOpenRelatedNote,
                    onToggleDeleted = { note ->
                        if (note.draftId != null) {
                            draftToDiscard = note
                        } else if (!note.isDeleted) {
                            showUndoForDeletedNote(note)
                        } else {
                            onToggleDeleted(note)
                        }
                    },
                    onReplyToNote = onReplyToNote,
                    sharedTransitionScope = sharedTransitionScope,
                    animatedVisibilityScope = navVisibilityScope,
                )
            }
        }
    }

    @Composable
    fun SharedUndoSnackbar(bottomInset: Dp) {
        AnimatedVisibility(
            visible = deletedNoteToUndo != null,
            enter = slideInVertically(initialOffsetY = { it }) + fadeIn(),
            exit = slideOutVertically(targetOffsetY = { it }) + fadeOut(),
            modifier = Modifier
                .padding(bottom = 16.dp + bottomInset)
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
    }

    @Composable
    fun SharedToolbarButtons(iconTint: Color) {
        if (selectedNoteIds.size == 1) {
            IconButton(
                onClick = {
                    val noteId = selectedNoteIds.first()
                    val note = displayNotes.find { it.id == noteId }

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
                        imageVector = Icons.Filled.Delete,
                        contentDescription = stringResource(R.string.delete),
                modifier = Modifier.size(24.dp),
                tint = if (selectedNoteIds.isNotEmpty()) iconTint else iconTint.copy(alpha = 0.38f)
            )
        }
    }

    @Composable
    fun SharedMultiSelectToolbar(bottomInset: Dp) {
        val isTabletLayout = context.resources.configuration.screenWidthDp >= 600
        AnimatedVisibility(
            visible = isSelectionMode,
            enter = fadeIn() + (if (isTabletLayout) slideInHorizontally(initialOffsetX = { -it }) else slideInVertically(initialOffsetY = { it })),
            exit = fadeOut() + (if (isTabletLayout) slideOutHorizontally(targetOffsetX = { -it }) else slideOutVertically(targetOffsetY = { it })),
            modifier = if (isTabletLayout) Modifier else Modifier.padding(bottom = 16.dp + bottomInset).offset(y = fabDodgeOffset)
        ) {
            HorizontalFloatingToolbar(
                expanded = true,
                colors = FloatingToolbarDefaults.standardFloatingToolbarColors(
                    toolbarContainerColor = MaterialTheme.colorScheme.secondaryContainer,
                    toolbarContentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                ),
            ) {
                SharedToolbarButtons(iconTint = MaterialTheme.colorScheme.onSecondaryContainer)
            }
        }
    }
    
    @Composable
    fun SharedFabContent(showComposeFab: Boolean = true, showTimelineFab: Boolean = true) {
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
                                noteGridState.animateScrollToItem(0)
                            }
                        },
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                    ) {
                    Icon(Icons.Filled.VerticalAlignTop, contentDescription = stringResource(R.string.backtop))
                    }
                }

                if (showTimelineFab && uiState.showTimelineJumpTool) {
                    FloatingActionButton(
                        onClick = {
                            showTimelineBrowser = true
                            onOpenTimelineBrowser()
                        },
                        containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                        contentColor = MaterialTheme.colorScheme.onTertiaryContainer,
                    ) {
                        Icon(Icons.Filled.CalendarMonth, contentDescription = "时间浏览")
                    }
                }

                if (showComposeFab) {
                    FloatingActionButton(
                        onClick = onComposeNote,
                        shape = CircleShape,
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
        }
    }

    // ==================== 渲染核心布局：判断屏幕宽度自适应 ====================
    BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
        val isTablet = maxWidth >= 600.dp

        if (isTablet) {
            Row(modifier = Modifier.fillMaxSize()) {
                val railState = rememberWideNavigationRailState(WideNavigationRailValue.Expanded)
                val isRailExpanded = railState.targetValue == WideNavigationRailValue.Expanded

                WideNavigationRail(
                    state = railState,
                    modifier = Modifier.fillMaxHeight(),
                    colors = WideNavigationRailDefaults.colors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant,
                    ),
                    header = {
                        Column(
                            modifier = Modifier.padding(top = 8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            IconButton(
                                modifier = Modifier.padding(start = 24.dp),
                                onClick = {
                                    scope.launch {
                                        if (railState.targetValue == WideNavigationRailValue.Expanded) {
                                            railState.collapse()
                                        } else {
                                            railState.expand()
                                        }
                                    }
                                },
                            ) {
                                Icon(
                                    imageVector = if (isRailExpanded) {
                                        Icons.AutoMirrored.Filled.MenuOpen
                                    } else {
                                        Icons.Filled.Menu
                                    },
                                    contentDescription = null,
                                )
                            }
                            AnimatedVisibility(visible = !isSelectionMode) {
                                Column(
                                    modifier = Modifier.padding(start = 20.dp),
                                    verticalArrangement = Arrangement.spacedBy(12.dp),
                                ) {
                                    if (isRailExpanded) {
                                        ExtendedFloatingActionButton(
                                            text = { Text(stringResource(R.string.edit_title_creat)) },
                                            icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                                            onClick = onComposeNote,
                                            containerColor = MaterialTheme.colorScheme.primaryContainer,
                                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                        )
                                    } else {
                                        FloatingActionButton(
                                            onClick = onComposeNote,
                                            containerColor = MaterialTheme.colorScheme.primaryContainer,
                                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                        ) {
                                            Icon(
                                                Icons.Filled.Add,
                                                contentDescription = stringResource(R.string.edit_title_creat),
                                            )
                                        }
                                    }
                                    if (uiState.showTimelineJumpTool) {
                                        if (isRailExpanded) {
                                            ExtendedFloatingActionButton(
                                                text = { Text("时间浏览") },
                                                icon = { Icon(Icons.Filled.CalendarMonth, contentDescription = null) },
                                                onClick = {
                                                    showTimelineBrowser = true
                                                    onOpenTimelineBrowser()
                                                },
                                                containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                                                contentColor = MaterialTheme.colorScheme.onTertiaryContainer,
                                            )
                                        } else {
                                            FloatingActionButton(
                                                onClick = {
                                                    showTimelineBrowser = true
                                                    onOpenTimelineBrowser()
                                                },
                                                containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                                                contentColor = MaterialTheme.colorScheme.onTertiaryContainer,
                                            ) {
                                                Icon(
                                                    Icons.Filled.CalendarMonth,
                                                    contentDescription = "时间浏览",
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxHeight()
                            .verticalScroll(rememberScrollState()),
                    ) {
                        WideNavigationRailItem(
                            selected = false,
                            onClick = onOpenSearch,
                            icon = { Icon(Icons.Filled.Search, contentDescription = null) },
                            label = { Text(stringResource(R.string.content_desc_search)) },
                            railExpanded = isRailExpanded,
                        )
                        WideNavigationRailItem(
                            selected = false,
                            onClick = onOpenTrash,
                            icon = { Icon(Icons.Filled.DeleteSweep, contentDescription = null) },
                            label = { Text(stringResource(R.string.trash_title)) },
                            railExpanded = isRailExpanded,
                        )
                        WideNavigationRailItem(
                            selected = false,
                            onClick = onOpenSettings,
                            icon = { Icon(Icons.Filled.Settings, contentDescription = null) },
                            label = { Text(stringResource(R.string.settings)) },
                            railExpanded = isRailExpanded,
                        )
                    }
                }

                Scaffold(
                    modifier = Modifier.weight(1f),
                    topBar = {
                        if (isSelectionMode) {
                            TopAppBar(
                                title = {
                                    Text(
                                        text = "${stringResource(R.string.selected)} ${selectedNoteIds.size}",
                                        style = MaterialTheme.typography.titleLarge,
                                        fontWeight = FontWeight.Bold
                                    )
                                },
                                navigationIcon = {
                                    IconButton(onClick = { isSelectionMode = false; selectedNoteIds = emptySet() }) {
                                        Icon(Icons.Filled.Close, null)
                                    }
                                }
                            )
                        }
                    },
                    floatingActionButton = { SharedFabContent(showComposeFab = false, showTimelineFab = false) }
                ) { padding ->
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(top = padding.calculateTopPadding())
                    ) {
                        Column(modifier = Modifier.fillMaxSize()) {
                            AnimatedVisibility(
                                visible = uiState.showTagBar && !uiState.isSearchMode && !isSelectionMode,
                                enter = expandVertically() + fadeIn(),
                                exit = shrinkVertically() + fadeOut()
                            ) {
                                SharedTags()
                            }

                            Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
                                SharedFeedArea(bottomInset = padding.calculateBottomPadding())
                            }
                        }

                        Box(modifier = Modifier.align(Alignment.BottomCenter)) {
                            SharedUndoSnackbar(bottomInset = padding.calculateBottomPadding())
                        }
                        Box(modifier = Modifier.align(Alignment.BottomCenter)) {
                            SharedMultiSelectToolbar(bottomInset = padding.calculateBottomPadding())
                        }
                    }
                }
            }
        } else {
            // ==================== 手机：原生 Scaffold 垂直布局 ====================
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
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        modifier = Modifier.horizontalScroll(rememberScrollState())
                                    ) {
                                        Text(
                                            text = "Synap",
                                            style = MaterialTheme.typography.titleLarge,
                                            fontWeight = FontWeight.Bold,
                                            color = MaterialTheme.colorScheme.primary
                                        )
                                        if (buildVersion.isNotBlank()) {
                                            val tag = detectVersionType(buildVersion)
                                            if (tag != VersionType.Release) {
                                                Spacer(modifier = Modifier.width(6.dp))
                                                Text(
                                                    text = tag.name,
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                                                    modifier = Modifier
                                                        .background(MaterialTheme.colorScheme.primaryContainer, RoundedCornerShape(4.dp))
                                                        .padding(horizontal = 6.dp, vertical = 2.dp),
                                                )
                                            }
                                        }
                                        Spacer(modifier = Modifier.width(8.dp))
                                    }
                                },
                                actions = {
                                    Row(
                                        modifier = Modifier.horizontalScroll(rememberScrollState()),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        AnimatedVisibility(
                                            visible = !isAtTop,
                                            enter = fadeIn(),
                                            exit = fadeOut()
                                        ) {
                                            IconButton(onClick = onOpenSearch) {
                                                Icon(Icons.Filled.Search, contentDescription = stringResource(R.string.content_desc_search))
                                            }
                                        }

                                        IconButton(onClick = onOpenScanner) {
                                            Icon(Icons.Filled.QrCodeScanner, contentDescription = "扫一扫")
                                        }

                                        IconButton(onClick = onOpenTrash) {
                                            Icon(Icons.Filled.DeleteSweep, contentDescription = stringResource(R.string.trash_title))
                                        }

                                        IconButton(onClick = onOpenSettings) {
                                            Icon(Icons.Filled.Settings, contentDescription = stringResource(R.string.content_desc_settings))
                                        }
                                    }
                                },
                            )

                            AnimatedVisibility(
                                visible = isAtTop,
                                enter = expandVertically() + fadeIn(),
                                exit = shrinkVertically() + fadeOut()
                            ) {
                                Box(
                                    modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)
                                ) {
                                    SharedSearchBox()
                                }
                            }
                        }
                    }
                },
                floatingActionButton = { SharedFabContent() }
            ) { innerPadding ->
                Box(
                    modifier = Modifier.fillMaxSize().padding(top = innerPadding.calculateTopPadding())
                ) {
                    Column(modifier = Modifier.fillMaxSize()) {
                        AnimatedVisibility(
                            visible = uiState.showTagBar && !uiState.isSearchMode && !isSelectionMode,
                            enter = expandVertically() + fadeIn(),
                            exit = shrinkVertically() + fadeOut()
                        ) {
                            SharedTags()
                        }

                        Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
                            SharedFeedArea(bottomInset = innerPadding.calculateBottomPadding())
                        }
                    }

                    Box(modifier = Modifier.align(Alignment.BottomCenter)) {
                        SharedUndoSnackbar(bottomInset = innerPadding.calculateBottomPadding())
                    }
                    Box(modifier = Modifier.align(Alignment.BottomCenter)) {
                        SharedMultiSelectToolbar(bottomInset = innerPadding.calculateBottomPadding())
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

@OptIn(ExperimentalMaterial3Api::class, ExperimentalSharedTransitionApi::class)
@Composable
private fun TimelineBrowserSheet(
    uiState: TimelineBrowserUiState,
    onDismiss: () -> Unit,
    onSelectDate: (LocalDate) -> Unit,
    sheetState: SheetState,
) {
    val quickDates = remember { buildTimelineQuickDates() }
    val titleFormatter = remember { DateTimeFormatter.ofPattern("M月d日") }
    val fullFormatter = remember { DateTimeFormatter.ofPattern("yyyy年M月d日") }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp)
                .padding(bottom = 28.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Text(
                text = "时间浏览",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "主面板已定位到 ${uiState.selectedDate.format(fullFormatter)} 附近",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            LazyRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                contentPadding = PaddingValues(vertical = 2.dp),
            ) {
                items(quickDates, key = { it.label }) { item ->
                    FilterChip(
                        selected = item.date == uiState.selectedDate,
                        onClick = { onSelectDate(item.date) },
                        label = { Text(item.label) },
                    )
                }
            }
            TimelineDensityWheel(
                uiState = uiState,
                onSelectDate = onSelectDate,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(180.dp),
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TextButton(onClick = { onSelectDate(uiState.selectedDate.minusDays(1)) }) {
                    Text("前一天")
                }
                Text(
                    text = uiState.selectedDate.format(titleFormatter),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                TextButton(onClick = { onSelectDate(uiState.selectedDate.plusDays(1)) }) {
                    Text("后一天")
                }
            }
        }
    }
}

@Composable
private fun TimelineDensityWheel(
    uiState: TimelineBrowserUiState,
    onSelectDate: (LocalDate) -> Unit,
    modifier: Modifier = Modifier,
) {
    val zone = remember { ZoneId.systemDefault() }
    val points = uiState.densityPoints
    val selectedDate = uiState.selectedDate
    val maxCount = points.maxOfOrNull { it.noteCount }?.coerceAtLeast(1) ?: 1
    val primary = MaterialTheme.colorScheme.primary
    val tertiary = MaterialTheme.colorScheme.tertiary
    val outline = MaterialTheme.colorScheme.outlineVariant
    val onSurfaceVariant = MaterialTheme.colorScheme.onSurfaceVariant

    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Box(modifier = modifier) {
            Canvas(
                modifier = Modifier
                    .fillMaxSize()
                    .pointerInput(points, selectedDate) {
                        detectTapGestures { offset ->
                            val date = nearestTimelinePointDate(
                                points = points,
                                offset = offset,
                                width = size.width.toFloat(),
                                height = size.height.toFloat(),
                                zone = zone,
                                requireCloseHit = true,
                            )
                            if (date != null) {
                                onSelectDate(date)
                            }
                        }
                    }
                    .pointerInput(points, selectedDate) {
                        var lastDragDate: LocalDate? = null
                        detectDragGestures(
                            onDragStart = { offset ->
                                lastDragDate = nearestTimelinePointDate(
                                    points = points,
                                    offset = offset,
                                    width = size.width.toFloat(),
                                    height = size.height.toFloat(),
                                    zone = zone,
                                    requireCloseHit = false,
                                )
                                lastDragDate?.let { date ->
                                    if (date != selectedDate) {
                                        onSelectDate(date)
                                    }
                                }
                            },
                            onDrag = { change, _ ->
                                val date = nearestTimelinePointDate(
                                    points = points,
                                    offset = change.position,
                                    width = size.width.toFloat(),
                                    height = size.height.toFloat(),
                                    zone = zone,
                                    requireCloseHit = false,
                                )
                                if (date != null && date != lastDragDate) {
                                    lastDragDate = date
                                    onSelectDate(date)
                                }
                            },
                        )
                    },
            ) {
                val centerX = size.width / 2f
                val centerY = size.height + 8f
                val radius = timelineWheelRadius(size.width, size.height)
                val selectedStart = selectedDate.atStartOfDay(zone).toInstant().toEpochMilli()
                val selectedEnd = selectedDate.plusDays(1).atStartOfDay(zone).toInstant().toEpochMilli()

                drawArc(
                    color = outline.copy(alpha = 0.55f),
                    startAngle = 180f,
                    sweepAngle = 180f,
                    useCenter = false,
                    topLeft = Offset(centerX - radius, centerY - radius),
                    size = androidx.compose.ui.geometry.Size(radius * 2f, radius * 2f),
                    style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round),
                )

                points.forEachIndexed { index, point ->
                    val angle = timelineWheelAngle(timelineWheelFraction(index, points.size))
                    val x = centerX + (cos(angle) * radius).toFloat()
                    val y = centerY - (kotlin.math.sin(angle) * radius).toFloat()
                    val normalized = point.noteCount / maxCount.toFloat()
                    val pointRadius = 4.dp.toPx() + 12.dp.toPx() * normalized
                    val isSelected = point.startedAt < selectedEnd && point.endedAt >= selectedStart
                    val color = if (isSelected) primary else if (point.noteCount > 0) tertiary else onSurfaceVariant.copy(alpha = 0.35f)

                    if (point.noteCount > 0) {
                        val barLength = 10.dp.toPx() + 22.dp.toPx() * normalized
                        val innerRadius = radius - barLength
                        val innerX = centerX + (cos(angle) * innerRadius).toFloat()
                        val innerY = centerY - (kotlin.math.sin(angle) * innerRadius).toFloat()
                        drawLine(
                            color = color.copy(alpha = if (isSelected) 0.92f else 0.5f),
                            start = Offset(innerX, innerY),
                            end = Offset(x, y),
                            strokeWidth = if (isSelected) 4.dp.toPx() else 2.dp.toPx(),
                            cap = StrokeCap.Round,
                        )
                    }

                    if (isSelected) {
                        drawCircle(
                            color = primary.copy(alpha = 0.14f),
                            radius = pointRadius + 8.dp.toPx(),
                            center = Offset(x, y),
                        )
                    }

                    drawCircle(
                        color = color.copy(alpha = if (point.noteCount > 0 || isSelected) 1f else 0.45f),
                        radius = if (isSelected) pointRadius + 3.dp.toPx() else pointRadius,
                        center = Offset(x, y),
                    )
                }
            }
            if (uiState.isDensityLoading) {
                CircularProgressIndicator(
                    modifier = Modifier
                        .align(Alignment.Center)
                        .size(28.dp),
                    strokeWidth = 2.dp,
                )
            }
        }
        val selectedCount = points.firstOrNull { point ->
            val selectedStart = selectedDate.atStartOfDay(zone).toInstant().toEpochMilli()
            val selectedEnd = selectedDate.plusDays(1).atStartOfDay(zone).toInstant().toEpochMilli()
            point.startedAt < selectedEnd && point.endedAt >= selectedStart
        }?.noteCount ?: 0
        Text(
            text = "${selectedDate.format(DateTimeFormatter.ofPattern("M月d日"))} · $selectedCount 条",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.align(Alignment.CenterHorizontally),
        )
        uiState.densityErrorMessage?.let { message ->
            Text(
                text = message,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.align(Alignment.CenterHorizontally),
            )
        }
    }
}

private data class TimelineQuickDate(
    val label: String,
    val date: LocalDate,
)

private fun nearestTimelinePointDate(
    points: List<com.synap.app.ui.viewmodel.TimelineDensityPoint>,
    offset: Offset,
    width: Float,
    height: Float,
    zone: ZoneId,
    requireCloseHit: Boolean,
): LocalDate? {
    if (points.isEmpty()) {
        return null
    }

    val centerX = width / 2f
    val centerY = height + 8f
    val radius = timelineWheelRadius(width, height)
    val hitRadius = 34f
    val nearest = points
        .mapIndexed { index, point ->
            val angle = timelineWheelAngle(timelineWheelFraction(index, points.size))
            val x = centerX + (cos(angle) * radius).toFloat()
            val y = centerY - (kotlin.math.sin(angle) * radius).toFloat()
            val distanceSquared = ((offset.x - x) * (offset.x - x)) + ((offset.y - y) * (offset.y - y))
            point to distanceSquared
        }
        .minByOrNull { it.second }
        ?: return null

    if (requireCloseHit && nearest.second > hitRadius * hitRadius) {
        return null
    }

    return Instant
        .ofEpochMilli(nearest.first.startedAt)
        .atZone(zone)
        .toLocalDate()
}

private fun timelineWheelRadius(width: Float, height: Float): Float =
    min(width * 0.44f, height * 0.92f)

private fun timelineWheelFraction(index: Int, count: Int): Float =
    if (count <= 1) 0.5f else index / (count - 1).toFloat()

private fun timelineWheelAngle(fraction: Float): Double =
    Math.toRadians(180.0 - (180.0 * fraction))

private fun buildTimelineQuickDates(): List<TimelineQuickDate> {
    val today = LocalDate.now()
    return listOf(
        TimelineQuickDate("今天", today),
        TimelineQuickDate("昨天", today.minusDays(1)),
        TimelineQuickDate("前天", today.minusDays(2)),
        TimelineQuickDate("一周前", today.minusWeeks(1)),
        TimelineQuickDate("一月前", today.minusMonths(1)),
    )
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
