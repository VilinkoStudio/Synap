package com.synap.app.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridItemSpan
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.TimelineSessionGroup
import com.synap.app.ui.util.formatSessionTimeRange

@Composable
fun HomeSessionFeed(
    sessions: List<TimelineSessionGroup>,
    state: LazyStaggeredGridState,
    isSelectionMode: Boolean, // 新增参数
    selectedNoteIds: Set<String>, // 新增参数
    onToggleSelection: (String) -> Unit, // 新增参数
    onEnterSelectionMode: (String) -> Unit, // 新增参数
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onReplyToNote: (String, String) -> Unit,
) {
    LazyVerticalStaggeredGrid(
        columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
        state = state,
        modifier = Modifier.fillMaxWidth().fillMaxHeight(),
        contentPadding = PaddingValues(
            start = 16.dp,
            top = 8.dp,
            end = 16.dp,
            bottom = 96.dp,
        ),
        verticalItemSpacing = 12.dp,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        sessions.forEachIndexed { sessionIndex, session ->
            item(span = StaggeredGridItemSpan.FullLine) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Text(
                        text = formatSessionTimeRange(session.startedAt, session.endedAt),
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = "${session.noteCount} 条笔记",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            itemsIndexed(
                items = session.notes,
                key = { _, note -> "${session.startedAt}_${note.id}_${note.isDeleted}" },
            ) { noteIndex, note ->
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
                    animationDelayMillis = ((sessionIndex + noteIndex).coerceAtMost(8)) * 35,
                )
            }
        }
    }
}