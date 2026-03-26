package com.fuwaki.synap.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Reply
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fuwaki.synap.ui.model.Note
import com.fuwaki.synap.ui.util.formatNoteTime
import com.fuwaki.synap.ui.viewmodel.DetailUiState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteDetailScreen(
    uiState: DetailUiState,
    onNavigateBack: () -> Unit,
    onDelete: () -> Unit,
    onReply: () -> Unit,
    onEdit: () -> Unit,
    onOpenRelatedNote: (String) -> Unit,
    onLoadMoreReplies: () -> Unit,
    onRefresh: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("笔记详情") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
                actions = {
                    IconButton(onClick = onDelete, enabled = uiState.note != null) {
                        Icon(Icons.Filled.Delete, contentDescription = "删除")
                    }
                },
            )
        },
    ) { innerPadding ->
        if (uiState.isLoading && uiState.note == null) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                CircularProgressIndicator()
            }
            return@Scaffold
        }

        if (uiState.note == null) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = uiState.errorMessage ?: "笔记不存在",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.error,
                )
                OutlinedButton(
                    onClick = onRefresh,
                    modifier = Modifier.padding(top = 16.dp),
                ) {
                    Text("重试")
                }
            }
            return@Scaffold
        }

        val note = uiState.note

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(bottom = 16.dp),
            ) {
                Text(
                    text = formatNoteTime(note.timestamp),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(end = 12.dp),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    note.tags.forEach { tag ->
                        Surface(
                            color = MaterialTheme.colorScheme.secondaryContainer,
                            shape = MaterialTheme.shapes.small,
                        ) {
                            Text(
                                text = tag,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                                style = MaterialTheme.typography.labelSmall,
                            )
                        }
                    }
                }
            }

            Text(
                text = note.content,
                style = MaterialTheme.typography.bodyLarge,
                lineHeight = 28.sp,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(20.dp))

            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                FilledTonalButton(onClick = onReply) {
                    Icon(Icons.Filled.Reply, contentDescription = null)
                    Text(" 回复")
                }
                FilledTonalButton(onClick = onEdit) {
                    Icon(Icons.Filled.Edit, contentDescription = null)
                    Text(" 编辑")
                }
            }

            if (uiState.errorMessage != null) {
                Text(
                    text = uiState.errorMessage,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(top = 16.dp),
                )
            }

            RelationSection(
                title = "父链溯源",
                notes = uiState.origins,
                onOpenRelatedNote = onOpenRelatedNote,
            )
            RelationSection(
                title = "前置版本",
                notes = uiState.previousVersions,
                onOpenRelatedNote = onOpenRelatedNote,
            )
            RelationSection(
                title = "后续版本",
                notes = uiState.nextVersions,
                onOpenRelatedNote = onOpenRelatedNote,
            )
            RelationSection(
                title = "回复流",
                notes = uiState.replies,
                onOpenRelatedNote = onOpenRelatedNote,
            )

            if (uiState.repliesHasMore) {
                OutlinedButton(
                    onClick = onLoadMoreReplies,
                    modifier = Modifier.padding(top = 12.dp),
                ) {
                    Text(if (uiState.repliesLoading) "加载中..." else "加载更多回复")
                }
            }

            Spacer(modifier = Modifier.height(48.dp))
        }
    }
}

@Composable
private fun RelationSection(
    title: String,
    notes: List<Note>,
    onOpenRelatedNote: (String) -> Unit,
) {
    if (notes.isEmpty()) {
        return
    }

    Text(
        text = title,
        style = MaterialTheme.typography.titleMedium,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(top = 24.dp, bottom = 12.dp),
    )

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        notes.forEach { note ->
            Surface(
                shape = MaterialTheme.shapes.medium,
                color = MaterialTheme.colorScheme.surfaceVariant,
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { onOpenRelatedNote(note.id) },
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text(
                        text = note.content,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    if (note.tags.isNotEmpty()) {
                        Text(
                            text = note.tags.joinToString(" · "),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 6.dp),
                        )
                    }
                }
            }
        }
    }
}
