package com.fuwaki.synap.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Reply
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope // --- 新增引入 ---
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.fuwaki.synap.LocalNoteTextSize
import com.fuwaki.synap.ui.model.Note
import com.fuwaki.synap.ui.util.formatNoteTime
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch // --- 新增引入 ---

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteCardItem(
    note: Note,
    onClick: () -> Unit,
    onToggleDeleted: () -> Unit,
    onReply: () -> Unit,
    animationDelayMillis: Int = 0,
) {
    val scope = rememberCoroutineScope() // --- 新增协程作用域 ---

    var entered by remember(note.id) { mutableStateOf(false) }
    LaunchedEffect(note.id) {
        if (animationDelayMillis > 0) {
            delay(animationDelayMillis.toLong())
        }
        entered = true
    }
    val cardAlpha by animateFloatAsState(
        targetValue = if (entered) 1f else 0f,
        animationSpec = tween(durationMillis = 320),
        label = "note_card_alpha",
    )
    val cardOffsetY by animateFloatAsState(
        targetValue = if (entered) 0f else 28f,
        animationSpec = tween(durationMillis = 380),
        label = "note_card_offset_y",
    )
    val cardScale by animateFloatAsState(
        targetValue = if (entered) 1f else 0.97f,
        animationSpec = tween(durationMillis = 320),
        label = "note_card_scale",
    )

    val dismissState = rememberSwipeToDismissBoxState(
        confirmValueChange = { dismissValue ->
            when (dismissValue) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    // --- 核心修复：延迟 150ms 触发数据更改，防止由于列表元素瞬间被移除导致的 Compose 闪退 ---
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
        enableDismissFromStartToEnd = true,
        enableDismissFromEndToStart = !note.isDeleted,
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
            modifier = Modifier
                .fillMaxWidth()
                .graphicsLayer {
                    alpha = cardAlpha
                    translationY = cardOffsetY
                    scaleX = cardScale
                    scaleY = cardScale
                }

                .clickable(
                    enabled = !note.isDeleted,
                    onClick = onClick
                ),
            colors = CardDefaults.cardColors(
                containerColor = if (note.isDeleted) {
                    MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.55f)
                } else {
                    MaterialTheme.colorScheme.surfaceVariant
                },
            ),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = note.content,
                    style = MaterialTheme.typography.bodyLarge.copy(
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
            }
        }
    }
}