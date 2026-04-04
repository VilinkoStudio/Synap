package com.synap.app.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.TimelineSessionGroup
import com.synap.app.ui.util.formatSessionTimeRange

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun HomeSessionFeed(
    sessions: List<TimelineSessionGroup>,
    state: LazyListState,
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onReplyToNote: (String, String) -> Unit,
) {
    LazyColumn(
        state = state,
        modifier = Modifier.fillMaxWidth(),
        contentPadding = PaddingValues(
            start = 16.dp,
            top = 8.dp,
            end = 16.dp,
            bottom = 96.dp,
        ),
        verticalArrangement = Arrangement.spacedBy(18.dp),
    ) {
        itemsIndexed(
            items = sessions,
            key = { _, session -> "${session.startedAt}_${session.endedAt}_${session.noteCount}" },
        ) { sessionIndex, session ->
            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(24.dp),
                color = MaterialTheme.colorScheme.surfaceContainerHigh,
                tonalElevation = 2.dp,
            ) {
                Column(
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
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

                    FlowRow(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        val itemWidth = if (session.notes.size == 1) 1f else 0.48f

                        session.notes.forEachIndexed { noteIndex, note ->
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth(itemWidth)
                                    .widthIn(max = 420.dp),
                            ) {
                                NoteCardItem(
                                    note = note,
                                    onClick = { onOpenNote(note.id) },
                                    onToggleDeleted = { onToggleDeleted(note) },
                                    onReply = { onReplyToNote(note.id, note.content) },
                                    animationDelayMillis = ((sessionIndex + noteIndex).coerceAtMost(8)) * 35,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
