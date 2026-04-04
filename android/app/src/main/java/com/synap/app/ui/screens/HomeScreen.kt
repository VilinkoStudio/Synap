package com.synap.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
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
import androidx.compose.runtime.saveable.rememberSaveable
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
import androidx.compose.ui.unit.sp
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
import java.util.Calendar
import kotlin.math.PI
import kotlin.math.sin

data class EasterEgg(
    val startMonth: Int,
    val startDate: Int,
    val endMonth: Int,
    val endDate: Int,
    val emoji: String,
    val dialogTitle: String,
    val dialogMessage: String
)

private val easterEggs = listOf(
    // 愚人节 (4月1日)
    EasterEgg(
        startMonth = Calendar.APRIL, startDate = 1,
        endMonth = Calendar.APRIL, endDate = 1,
        emoji = "🤡",
        dialogTitle = "愚人节彩蛋",
        dialogMessage = "你被骗了！愚人节快乐！"
    ),
    // 清明节 (4月2日 - 4月5日)
    EasterEgg(
        startMonth = Calendar.APRIL, startDate = 2,
        endMonth = Calendar.APRIL, endDate = 5,
        emoji = "🌱",
        dialogTitle = "清明节彩蛋",
        dialogMessage = "“清明时节雨纷纷，路上行人欲断魂。”——杜牧《清明》。注意休息，踏青愉快。"
    ),
    // 劳动节 (5月1日)
    EasterEgg(
        startMonth = Calendar.MAY, startDate = 1,
        endMonth = Calendar.MAY, endDate = 5,
        emoji = "🛠️",
        dialogTitle = "劳动节彩蛋",
        dialogMessage = "劳动节快乐，感谢你的辛勤付出！劳动节小长假记得出门开开风景。"
    ),
    // 元旦 (1月1日)
    EasterEgg(
        startMonth = Calendar.JANUARY, startDate = 1,
        endMonth = Calendar.JANUARY, endDate = 1,
        emoji = "🎉",
        dialogTitle = "新年快乐",
        dialogMessage = "新的一年，新的开始！祝你新的一年，所愿皆成真。"
    ),
    // 国庆 (9月30日 - 10月7日)
    EasterEgg(
        startMonth = Calendar.SEPTEMBER, startDate = 30,
        endMonth = Calendar.OCTOBER, endDate = 7,
        emoji = "🎉",
        dialogTitle = "国庆节快乐",
        dialogMessage = "十一小长假有规划去哪里玩吗？没有的话要不用 Synap 规划一下，嘻嘻。"
    )
)

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
    val sessionListState = rememberLazyListState()
    val scope = rememberCoroutineScope()

    var deletedNoteToUndo by remember { mutableStateOf<Note?>(null) }
    var undoProgress by remember { mutableFloatStateOf(1f) }
    var timeLeftSeconds by remember { mutableIntStateOf(3) }

    var pendingDeleteNoteIds by remember { mutableStateOf(setOf<String>()) }

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

    val currentEasterEgg by remember {
        val calendar = Calendar.getInstance()
        val currentMonth = calendar.get(Calendar.MONTH)
        val currentDay = calendar.get(Calendar.DAY_OF_MONTH)

        val matchedEgg = easterEggs.firstOrNull { egg ->
            if (egg.startMonth == egg.endMonth) {
                currentMonth == egg.startMonth && currentDay in egg.startDate..egg.endDate
            } else {
                (currentMonth == egg.startMonth && currentDay >= egg.startDate) ||
                        (currentMonth == egg.endMonth && currentDay <= egg.endDate) ||
                        (currentMonth in (egg.startMonth + 1)..<egg.endMonth)
            }
        }
        mutableStateOf(matchedEgg)
    }

    var showEasterEggDialog by remember { mutableStateOf(false) }

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

    val isScrolledDown by remember(noteGridState, sessionListState, isShowingSessionFeed) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                sessionListState.firstVisibleItemIndex > 0 || sessionListState.firstVisibleItemScrollOffset > 100
            } else {
                noteGridState.firstVisibleItemIndex > 0 || noteGridState.firstVisibleItemScrollOffset > 100
            }
        }
    }

    val isAtTop by remember(noteGridState, sessionListState, isShowingSessionFeed) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                sessionListState.firstVisibleItemIndex == 0 && sessionListState.firstVisibleItemScrollOffset <= 10
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

    val shouldLoadMore by remember(
        noteGridState,
        sessionListState,
        displayNotes,
        displaySessionGroups,
        uiState.hasMore,
        uiState.isLoading,
        isShowingSessionFeed,
    ) {
        derivedStateOf {
            if (isShowingSessionFeed) {
                val lastVisible = sessionListState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
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

    val allTags = uiState.availableTags
    val isActiveFeedEmpty = if (isShowingSessionFeed) {
        displaySessionGroups.isEmpty()
    } else {
        displayNotes.isEmpty()
    }

    LaunchedEffect(
        noteGridState,
        sessionListState,
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

    if (showEasterEggDialog && currentEasterEgg != null) {
        AlertDialog(
            onDismissRequest = { showEasterEggDialog = false },
            title = { Text(currentEasterEgg!!.dialogTitle) },
            text = { Text(currentEasterEgg!!.dialogMessage) },
            confirmButton = {
                TextButton(onClick = { showEasterEggDialog = false }) {
                    Text("好的")
                }
            }
        )
    }

    Scaffold(
        topBar = {
            Column {
                TopAppBar(
                    title = {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                text = "Synap",
                                style = MaterialTheme.typography.titleLarge,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.primary
                            )

                            AnimatedVisibility(
                                visible = currentEasterEgg != null,
                                enter = fadeIn() + scaleIn(),
                                exit = fadeOut() + scaleOut()
                            ) {
                                IconButton(
                                    onClick = {
                                        showEasterEggDialog = true
                                    },
                                    modifier = Modifier
                                        .padding(start = 4.dp)
                                        .size(36.dp)
                                ) {
                                    Text(currentEasterEgg?.emoji ?: "", fontSize = 22.sp)
                                }
                            }
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

                        IconButton(onClick = onOpenTrash) {
                            Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.trash_title))
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
                                    text = stringResource(R.string.home_search_title),
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
                                    sessionListState.animateScrollToItem(0)
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

                FloatingActionButton(onClick = onComposeNote) {
                    Icon(Icons.Filled.Add, contentDescription = stringResource(R.string.home_creatnote))
                }
            }
        },
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            Column(
                modifier = Modifier.fillMaxSize(),
            ) {
                HomeFilterBar(
                    isFilterPanelOpen = uiState.isFilterPanelOpen,
                    isTagsExpanded = isTagsExpanded,
                    allTags = allTags,
                    unselectedTags = uiState.unselectedTags,
                    isUntaggedUnselected = uiState.isUntaggedUnselected,
                    onToggleFilterPanel = onSetFilterPanelOpen,
                    onToggleTagsExpanded = { isTagsExpanded = !isTagsExpanded },
                    onToggleAllTags = onToggleAllTags,
                    onToggleUntaggedFilter = onToggleUntaggedFilter,
                    onToggleTagFilter = onToggleTagFilter,
                )

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
                                text = "这里空空如也",
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
                                    state = sessionListState,
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
                                Text("已删除笔记", style = MaterialTheme.typography.bodyMedium)
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(
                                    text = "${timeLeftSeconds}s",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.primary,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                            Text(
                                text = "撤销删除",
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

private val sampleNotes = listOf(
    Note(
        id = "1",
        content = "今天的会议讨论了产品路线图，需要注意几个关键决策点。",
        tags = listOf("会议", "产品"),
        timestamp = 1700000000000,
    ),
    Note(
        id = "2",
        content = "读书笔记：设计模式中的策略模式可以用来替换多重条件判断。",
        tags = listOf("读书", "技术"),
        timestamp = 1699900000000,
    ),
    Note(
        id = "3",
        content = "买了新的笔记本支架，站立办公效果不错。",
        tags = listOf("生活"),
        timestamp = 1699800000000,
    ),
    Note(
        id = "4",
        content = "灵感：可以做一个碎片化知识管理工具，自动关联相关内容。",
        tags = listOf("灵感", "产品"),
        timestamp = 1699700000000,
        isDeleted = true,
    ),
)

private val sampleSessionGroups = listOf(
    TimelineSessionGroup(
        startedAt = 1700000000000,
        endedAt = 1700003600000,
        noteCount = 2,
        notes = sampleNotes.take(2),
    ),
    TimelineSessionGroup(
        startedAt = 1699900000000,
        endedAt = 1699901800000,
        noteCount = 2,
        notes = sampleNotes.drop(2),
    ),
)

@Preview(name = "With data", showBackground = true)
@Composable
private fun HomeScreenPreview() {
    MyApplicationTheme {
        HomeScreen(
            uiState = HomeUiState(
                isLoading = false,
                notes = sampleNotes,
                sessionGroups = sampleSessionGroups,
                hasMore = true,
            ),
            onOpenSettings = {},
            onComposeNote = {},
            onOpenNote = {},
            onReplyToNote = { _, _ -> },
            onToggleDeleted = {},
            onOpenSearch = {},
            onOpenTrash = {},
            onLoadMore = {},
            onRefresh = {},
            onSetFilterPanelOpen = {},
            onToggleTagFilter = {},
            onToggleUntaggedFilter = {},
            onToggleAllTags = {},
        )
    }
}

@Preview(name = "Empty", showBackground = true)
@Composable
private fun HomeScreenEmptyPreview() {
    MyApplicationTheme {
        HomeScreen(
            uiState = HomeUiState(isLoading = false),
            onOpenSettings = {},
            onComposeNote = {},
            onOpenNote = {},
            onReplyToNote = { _, _ -> },
            onToggleDeleted = {},
            onOpenSearch = {},
            onOpenTrash = {},
            onLoadMore = {},
            onRefresh = {},
            onSetFilterPanelOpen = {},
            onToggleTagFilter = {},
            onToggleUntaggedFilter = {},
            onToggleAllTags = {},
        )
    }
}

@Preview(name = "Loading", showBackground = true)
@Composable
private fun HomeScreenLoadingPreview() {
    MyApplicationTheme {
        HomeScreen(
            uiState = HomeUiState(isLoading = true),
            onOpenSettings = {},
            onComposeNote = {},
            onOpenNote = {},
            onReplyToNote = { _, _ -> },
            onToggleDeleted = {},
            onOpenSearch = {},
            onOpenTrash = {},
            onLoadMore = {},
            onRefresh = {},
            onSetFilterPanelOpen = {},
            onToggleTagFilter = {},
            onToggleUntaggedFilter = {},
            onToggleAllTags = {},
        )
    }
}
