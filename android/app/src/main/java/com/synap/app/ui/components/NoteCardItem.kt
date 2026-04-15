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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.ParagraphStyle
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteTextSize
import com.synap.app.ui.model.Note
import com.synap.app.ui.util.formatNoteTime
import kotlinx.coroutines.launch

// ==================== 共享 Markdown 渲染引擎 ====================
fun buildMarkdownAnnotatedString(
    text: String,
    primaryColor: Color,
    highlightColor: Color,
    baseFontSize: Float,
    isCompact: Boolean = false
): AnnotatedString {
    val charArray = text.toCharArray()
    Regex("^(> )", RegexOption.MULTILINE).findAll(text).forEach { charArray[it.range.first] = '“' }
    Regex("^-\\s+\\[ \\]\\s", RegexOption.MULTILINE).findAll(text).forEach { match ->
        charArray[match.range.first] = '☐'
        for (i in match.range.first + 1..match.range.last) charArray[i] = ' '
    }
    Regex("^-\\s+\\[x\\]\\s", RegexOption.MULTILINE).findAll(text).forEach { match ->
        charArray[match.range.first] = '☑'
        for (i in match.range.first + 1..match.range.last) charArray[i] = ' '
    }
    if (!isCompact) {
        Regex("^- (?!(\\[ \\]|\\[x\\]))", RegexOption.MULTILINE).findAll(text).forEach { charArray[it.range.first] = '•' }
    }
    val visualString = String(charArray)

    return buildAnnotatedString {
        append(visualString)
        val hiddenSpanStyle = SpanStyle(color = Color.Transparent, fontSize = 0.1.sp)

        fun processMatches(regex: Regex, style: SpanStyle) {
            regex.findAll(visualString).forEach { match ->
                if (match.groups.size >= 2) {
                    val content = match.groups[1]!!
                    addStyle(style, content.range.first, content.range.last + 1)
                    addStyle(hiddenSpanStyle, match.range.first, content.range.first)
                    addStyle(hiddenSpanStyle, content.range.last + 1, match.range.last + 1)
                }
            }
        }

        processMatches(Regex("(?<!\\*)\\*(?!\\*)(.*?)(?<!\\*)\\*(?!\\*)"), SpanStyle(fontStyle = FontStyle.Italic))
        processMatches(Regex("~~(.*?)~~"), SpanStyle(textDecoration = TextDecoration.LineThrough))
        processMatches(Regex("<u>(.*?)</u>"), SpanStyle(textDecoration = TextDecoration.Underline))
        processMatches(Regex("==(.*?)=="), SpanStyle(background = highlightColor, color = Color.Black))

        Regex("^☐(     )", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
            addStyle(SpanStyle(color = primaryColor, fontSize = (baseFontSize * 1.3f).sp), match.range.first, match.range.first + 1)
            addStyle(hiddenSpanStyle, match.groups[1]!!.range.first, match.groups[1]!!.range.last + 1)
        }
        Regex("^☑(     )", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
            addStyle(SpanStyle(color = primaryColor, fontSize = (baseFontSize * 1.3f).sp), match.range.first, match.range.first + 1)
            addStyle(hiddenSpanStyle, match.groups[1]!!.range.first, match.groups[1]!!.range.last + 1)
        }

        if (!isCompact) {
            processMatches(Regex("\\*\\*\\*(.*?)\\*\\*\\*"), SpanStyle(fontWeight = FontWeight.Bold, fontStyle = FontStyle.Italic))
            processMatches(Regex("(?<!\\*)\\*\\*(?!\\*)(.*?)(?<!\\*)\\*\\*(?!\\*)"), SpanStyle(fontWeight = FontWeight.Bold))

            Regex("^(#{1,4} )(.*)", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                if (match.groups.size >= 3) {
                    val level = match.groups[1]!!.value.trim().length
                    val scale = 1.8f - (level * 0.15f)
                    addStyle(hiddenSpanStyle, match.groups[1]!!.range.first, match.groups[1]!!.range.last + 1)
                    val lineEnd = visualString.indexOf('\n', match.range.last).takeIf { it != -1 } ?: visualString.length
                    addStyle(SpanStyle(fontWeight = FontWeight.ExtraBold, fontSize = (baseFontSize * scale).sp, color = primaryColor), match.groups[2]!!.range.first, match.groups[2]!!.range.last + 1)
                    addStyle(ParagraphStyle(lineHeight = (baseFontSize * 1.5f).sp), match.range.first, match.range.last + 1)
                }
            }

            val lines = visualString.split('\n')
            var offset = 0
            var inQuote = false
            var quoteStart = 0

            for (i in lines.indices) {
                val line = lines[i]
                val lineLength = line.length

                if (line.startsWith("“ ")) {
                    if (!inQuote) {
                        inQuote = true
                        quoteStart = offset
                        addStyle(SpanStyle(color = Color.Gray, fontSize = (baseFontSize * 1.5f).sp, fontWeight = FontWeight.Black), offset, offset + 1)
                        addStyle(hiddenSpanStyle, offset + 1, offset + 2)
                    } else {
                        addStyle(hiddenSpanStyle, offset, offset + 2)
                    }
                    addStyle(SpanStyle(color = Color.Gray), offset + 2, offset + lineLength)
                } else {
                    if (inQuote) {
                        inQuote = false
                        addStyle(ParagraphStyle(lineHeight = (baseFontSize * 1.5f).sp), quoteStart, offset)
                    }
                }
                offset += lineLength + 1
            }
            if (inQuote) {
                addStyle(ParagraphStyle(lineHeight = (baseFontSize * 1.5f).sp), quoteStart, offset - 1)
            }

            Regex("^•( )", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                addStyle(SpanStyle(color = primaryColor, fontWeight = FontWeight.Bold), match.range.first, match.range.first + 1)
            }
        } else {
            Regex("\\*\\*\\*|\\*\\*").findAll(visualString).forEach { match ->
                addStyle(hiddenSpanStyle, match.range.first, match.range.last + 1)
            }
            Regex("^(#{1,4} )", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                addStyle(hiddenSpanStyle, match.range.first, match.range.last + 1)
            }
            Regex("^“( )", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                addStyle(hiddenSpanStyle, match.range.first, match.range.last + 1)
            }
            Regex("^>+ ", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                addStyle(hiddenSpanStyle, match.range.first, match.range.last + 1)
            }
            Regex("^(-\\s+|\\d+\\.\\s+)", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                addStyle(hiddenSpanStyle, match.range.first, match.range.last + 1)
            }
        }
    }
}

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

    // 【核心修复 1】：使用底层触摸 API，记录用户是否处于长按/滑动未松手状态
    var isPressed by remember { mutableStateOf(false) }

    val dismissState = rememberSwipeToDismissBoxState(
        confirmValueChange = { dismissValue ->
            if (isSelectionMode) return@rememberSwipeToDismissBoxState false // 多选模式下禁用滑动

            // 仅放行状态变化（让底色变红），绝不在这里执行删除逻辑！
            when (dismissValue) {
                SwipeToDismissBoxValue.StartToEnd -> true
                SwipeToDismissBoxValue.EndToStart -> !note.isDeleted
                SwipeToDismissBoxValue.Settled -> true
            }
        },
        // 【核心修复 2】：滑动 20% 即可触发状态（背景变色）
        positionalThreshold = { totalDistance -> totalDistance * 0.2f }
    )

    // 【核心修复 3】：只有等用户彻底松手（isPressed = false）并且状态已切换，才执行真正动作
    LaunchedEffect(dismissState.currentValue, isPressed) {
        if (!isPressed) {
            when (dismissState.currentValue) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    onToggleDeleted()
                    dismissState.snapTo(SwipeToDismissBoxValue.Settled)
                }
                SwipeToDismissBoxValue.EndToStart -> {
                    if (!note.isDeleted) {
                        onReply()
                    }
                    dismissState.snapTo(SwipeToDismissBoxValue.Settled)
                }
                SwipeToDismissBoxValue.Settled -> {}
            }
        }
    }

    SwipeToDismissBox(
        state = dismissState,
        enableDismissFromStartToEnd = !isSelectionMode,
        enableDismissFromEndToStart = !note.isDeleted && !isSelectionMode,
        modifier = Modifier.pointerInput(Unit) {
            awaitPointerEventScope {
                while (true) {
                    val event = awaitPointerEvent(PointerEventPass.Initial)
                    // 只要有任何一根手指按在屏幕上，就是 true，松手就是 false
                    isPressed = event.changes.any { it.pressed }
                }
            }
        },
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
                    isSelected -> MaterialTheme.colorScheme.secondaryContainer
                    else -> MaterialTheme.colorScheme.surfaceVariant
                }
            ),
        ) {
            Row(
                modifier = Modifier.padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    val primaryColor = MaterialTheme.colorScheme.primary
                    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
                    val baseFontSize = LocalNoteTextSize.current.value

                    // 使用共享渲染引擎（Compact 紧凑模式）
                    val annotatedContent = remember(note.content, primaryColor, highlightColor, baseFontSize) {
                        buildMarkdownAnnotatedString(note.content, primaryColor, highlightColor, baseFontSize, isCompact = true)
                    }

                    Text(
                        text = annotatedContent,
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