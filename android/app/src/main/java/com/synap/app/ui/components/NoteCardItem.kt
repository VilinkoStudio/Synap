package com.synap.app.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Reply
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteTextSize
import com.synap.app.ui.model.Note
import com.synap.app.ui.util.formatNoteTime
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun NoteCardItem(
    note: Note,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
    isSelectionMode: Boolean,
    isSelected: Boolean,
    onToggleDeleted: () -> Unit,
    onReply: () -> Unit,
    animationDelayMillis: Int = 0,
) {
    val scope = rememberCoroutineScope()

    val dismissState = rememberSwipeToDismissBoxState(
        confirmValueChange = { dismissValue ->
            if (isSelectionMode) return@rememberSwipeToDismissBoxState false // 多选模式下禁用滑动
            when (dismissValue) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    scope.launch {
                        delay(150)
                        onToggleDeleted()
                    }
                    false
                }
                SwipeToDismissBoxValue.EndToStart -> {
                    if (!note.isDeleted) {
                        scope.launch {
                            delay(150)
                            onReply()
                        }
                    }
                    false
                }
                SwipeToDismissBoxValue.Settled -> false
            }
        },
    )

    SwipeToDismissBox(
        state = dismissState,
        enableDismissFromStartToEnd = !isSelectionMode,
        enableDismissFromEndToStart = !note.isDeleted && !isSelectionMode,
        backgroundContent = {
            val color by animateColorAsState(
                targetValue = when (dismissState.targetValue) {
                    SwipeToDismissBoxValue.StartToEnd -> if (note.isDeleted) {
                        MaterialTheme.colorScheme.primaryContainer
                    } else {
                        MaterialTheme.colorScheme.errorContainer
                    }
                    SwipeToDismissBoxValue.EndToStart -> MaterialTheme.colorScheme.primaryContainer
                    SwipeToDismissBoxValue.Settled -> Color.Transparent
                },
                label = "dismiss_color",
            )

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(color, RoundedCornerShape(12.dp))
                    .padding(horizontal = 20.dp),
            ) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.CenterStart,
                ) {
                    Icon(
                        imageVector = if (note.isDeleted) Icons.Filled.Refresh else Icons.Filled.Delete,
                        contentDescription = null,
                    )
                }
                if (!note.isDeleted) {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.CenterEnd,
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Reply,
                            contentDescription = null,
                        )
                    }
                }
            }
        },
    ) {
        Card(
            modifier = modifier
                .fillMaxWidth()
                .combinedClickable(
                    enabled = !note.isDeleted || isSelectionMode,
                    onClick = onClick,
                    onLongClick = onLongClick
                ),
            colors = CardDefaults.cardColors(
                containerColor = when {
                    note.isDeleted -> MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.55f)
                    isSelected -> MaterialTheme.colorScheme.secondaryContainer // 选中时变为更深的 SecondaryContainer 色彩
                    else -> MaterialTheme.colorScheme.surfaceVariant
                }
            ),
        ) {
            Row(
                modifier = Modifier.padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = note.content,
                        style = MaterialTheme.typography.bodyLarge.copy(
                            fontFamily = LocalNoteFontFamily.current,
                            fontWeight = LocalNoteFontWeight.current,
                            fontSize = LocalNoteTextSize.current,
                            lineHeight = LocalNoteTextSize.current * 1.5f
                        ),
                        color = if (note.isDeleted) Color.Gray else Color.Unspecified,
                        textDecoration = if (note.isDeleted) TextDecoration.LineThrough else TextDecoration.None,
                        maxLines = 4,
                        overflow = TextOverflow.Ellipsis,
                    )

                    if (!note.parentSummary.isNullOrBlank()) {
                        Text(
                            text = "回复自“${note.parentSummary}”",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = formatNoteTime(note.timestamp),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(end = 12.dp),
                        )
                        Row(
                            modifier = Modifier.horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            note.tags.take(5).forEach { tag ->
                                Surface(
                                    color = if (isSelected) MaterialTheme.colorScheme.surfaceVariant else MaterialTheme.colorScheme.secondaryContainer,
                                    shape = MaterialTheme.shapes.small,
                                ) {
                                    Text(
                                        text = tag,
                                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                                        style = MaterialTheme.typography.labelSmall,
                                    )
                                }
                            }
                            if (note.tags.size > 5) {
                                Text(
                                    text = "+${note.tags.size - 5}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.padding(vertical = 4.dp),
                                )
                            }
                        }
                    }
                }

                // 多选模式下的复选框
                AnimatedVisibility(visible = isSelectionMode) {
                    Checkbox(
                        checked = isSelected,
                        onCheckedChange = null,
                        modifier = Modifier.padding(start = 16.dp)
                    )
                }
            }
        }
    }
}