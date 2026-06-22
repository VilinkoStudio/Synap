package com.synap.app.ui.components

import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridItemSpan
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.synap.app.ui.model.Note
import com.synap.app.ui.util.formatSessionTimeRange

@OptIn(ExperimentalSharedTransitionApi::class)
@Composable
fun HomeTimelineFeed(
    notes: List<Note>,
    showTimeGroups: Boolean,
    state: LazyStaggeredGridState,
    isSelectionMode: Boolean,
    selectedNoteIds: Set<String>,
    onToggleSelection: (String) -> Unit,
    onEnterSelectionMode: (String) -> Unit,
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onReplyToNote: (String, String) -> Unit,
    hasMore: Boolean,
    isLoadingMore: Boolean,
    onLoadMore: () -> Unit,
    bottomInset: Dp = 0.dp,
    sharedTransitionScope: SharedTransitionScope? = null,
    animatedVisibilityScope: AnimatedVisibilityScope? = null,
) {
    LazyVerticalStaggeredGrid(
        columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
        state = state,
        modifier = Modifier
            .fillMaxSize()
            .fillMaxWidth(),
        contentPadding = PaddingValues(
            start = 16.dp,
            top = 8.dp,
            end = 16.dp,
            bottom = 96.dp + bottomInset,
        ),
        verticalItemSpacing = 16.dp,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        notes.forEachIndexed { index, note ->
            if (showTimeGroups && startsTimelineGroup(index, note)) {
                val group = note.timelineGroup
                val startedAt = group?.startedAt ?: note.timestamp
                val endedAt = group?.endedAt ?: note.timestamp
                val noteCount = group?.noteCount ?: 1
                item(
                    key = "timeline_header_${startedAt}_${endedAt}_${note.id}",
                    span = StaggeredGridItemSpan.FullLine,
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text(
                            text = formatSessionTimeRange(startedAt, endedAt),
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = "${noteCount} 条笔记",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            item(key = "${note.id}_${note.isDeleted}") {
                NoteCardItem(
                    note = note,
                    onClick = {
                        if (isSelectionMode) onToggleSelection(note.id) else onOpenNote(note.id)
                    },
                    onLongClick = {
                        if (!isSelectionMode) onEnterSelectionMode(note.id)
                    },
                    isSelectionMode = isSelectionMode,
                    isSelected = selectedNoteIds.contains(note.id),
                    onToggleDeleted = { onToggleDeleted(note) },
                    onReply = { onReplyToNote(note.id, note.content) },
                    animationDelayMillis = (index.coerceAtMost(8)) * 35,
                    sharedTransitionScope = sharedTransitionScope,
                    animatedVisibilityScope = animatedVisibilityScope,
                )
            }
        }

        if (hasMore) {
            item(
                key = "timeline_load_more_sentinel",
                span = StaggeredGridItemSpan.FullLine,
            ) {
                LaunchedEffect(notes.size, isLoadingMore) {
                    if (!isLoadingMore) {
                        onLoadMore()
                    }
                }
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 20.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    CircularProgressIndicator(strokeWidth = 2.dp)
                }
            }
        }
    }
}

private fun startsTimelineGroup(index: Int, note: Note): Boolean {
    val group = note.timelineGroup ?: return false
    return group.startsGroup || index == 0
}
