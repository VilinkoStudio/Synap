package com.synap.app.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.synap.app.ui.model.Note

@Composable
fun HomeNoteFeed(
    notes: List<Note>,
    state: LazyStaggeredGridState,
    isSelectionMode: Boolean,
    selectedNoteIds: Set<String>,
    onToggleSelection: (String) -> Unit,
    onEnterSelectionMode: (String) -> Unit,
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onReplyToNote: (String, String) -> Unit,
    bottomInset: Dp = 0.dp // 统一传 Dp
) {
    LazyVerticalStaggeredGrid(
        columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
        state = state,
        modifier = Modifier.fillMaxWidth(),
        contentPadding = PaddingValues(
            start = 16.dp,
            top = 8.dp,
            end = 16.dp,
            bottom = 96.dp + bottomInset,
        ),
        verticalItemSpacing = 16.dp,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        itemsIndexed(notes, key = { _, note -> "${note.id}_${note.isDeleted}" }) { index, note ->
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
                animationDelayMillis = (index.coerceAtMost(6)) * 45,
            )
        }
    }
}