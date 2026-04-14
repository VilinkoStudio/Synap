package com.synap.app.ui.screens

import android.content.ActivityNotFoundException
import android.content.Intent
import android.provider.CalendarContract
import android.widget.Toast
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Reply
import androidx.compose.material.icons.filled.VerticalAlignTop
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalFloatingToolbar
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.ParagraphStyle
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteLineSpacing
import com.synap.app.LocalNoteTextSize
import com.synap.app.R
import com.synap.app.ui.model.Note
import com.synap.app.ui.util.formatNoteTime
import com.synap.app.ui.viewmodel.DetailUiState
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

@Suppress("OPT_IN_USAGE", "OPT_IN_USAGE_FUTURE_ERROR")
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun NoteDetailScreen(
    uiState: DetailUiState,
    onNavigateBack: () -> Unit,
    onNavigateHome: () -> Unit,
    onDelete: () -> Unit,
    onReply: () -> Unit,
    onEdit: () -> Unit,
    onOpenRelatedNote: (String) -> Unit,
    onLoadMoreReplies: () -> Unit,
    onRefresh: () -> Unit,
) {
    val scrollState = rememberScrollState()
    val scope = rememberCoroutineScope()
    val clipboardManager = LocalClipboardManager.current
    val context = LocalContext.current
    val addCalendarReminderLabel = stringResource(R.string.notedetail_add_calendar_reminder)
    val defaultCalendarTitle = stringResource(R.string.notedetail_calendar_default_title)
    val calendarUnavailableMessage = stringResource(R.string.notedetail_calendar_unavailable)

    val isScrolledDown by remember {
        derivedStateOf {
            scrollState.value > 300
        }
    }

    var showDeleteDialog by remember { mutableStateOf(false) }
    var showCopyDialog by remember { mutableStateOf(false) }

    // 删除弹窗
    if (showDeleteDialog) {
        AlertDialog(
            onDismissRequest = { showDeleteDialog = false },
            title = { Text("确认删除") },
            text = { Text("确定要删除这条笔记吗？") },
            confirmButton = {
                TextButton(onClick = {
                    showDeleteDialog = false
                    onDelete()
                }) {
                    Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteDialog = false }) { Text("取消") }
            }
        )
    }

    // 复制 Markdown 提示弹窗
    if (showCopyDialog && uiState.note != null) {
        AlertDialog(
            onDismissRequest = { showCopyDialog = false },
            title = { Text("复制选项") },
            text = { Text("检测到正文包含 Markdown 语法，请选择复制纯文本还是完整格式？") },
            confirmButton = {
                TextButton(onClick = {
                    clipboardManager.setText(AnnotatedString(uiState.note!!.content))
                    showCopyDialog = false
                }) {
                    Text("完整格式", color = MaterialTheme.colorScheme.primary)
                }
            },
            dismissButton = {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = { showCopyDialog = false }) {
                        Text("取消", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    TextButton(onClick = {
                        val plainText = uiState.note!!.content
                            .replace(Regex("\\*\\*(.*?)\\*\\*"), "$1")
                            .replace(Regex("(?<!\\*)\\*(?!\\*)(.*?)(?<!\\*)\\*(?!\\*)"), "$1")
                            .replace(Regex("~~(.*?)~~"), "$1")
                            .replace(Regex("<u>(.*?)</u>"), "$1")
                            .replace(Regex("==(.*?)=="), "$1")
                            .replace(Regex("^(#{1,4} )", RegexOption.MULTILINE), "")
                            .replace(Regex("^(>+ )", RegexOption.MULTILINE), "")
                        clipboardManager.setText(AnnotatedString(plainText))
                        showCopyDialog = false
                    }) {
                        Text("纯文本", color = MaterialTheme.colorScheme.primary)
                    }
                }
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.notedetail_title)) },
                navigationIcon = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        IconButton(onClick = onNavigateBack) {
                            Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                        }
                        IconButton(onClick = onNavigateHome) {
                            Icon(Icons.Filled.Home, contentDescription = stringResource(R.string.home))
                        }
                    }
                },
            )
        },
        floatingActionButton = {
            if (uiState.note != null) {
                AnimatedVisibility(
                    visible = isScrolledDown,
                    enter = fadeIn(),
                    exit = fadeOut()
                ) {
                    FloatingActionButton(
                        onClick = {
                            scope.launch {
                                scrollState.animateScrollTo(0)
                            }
                        },
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                        modifier = Modifier.padding(bottom = 80.dp)
                    ) {
                        Icon(Icons.Filled.VerticalAlignTop, contentDescription = stringResource(R.string.backtop))
                    }
                }
            }
        }
    ) { innerPadding ->

        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            if (uiState.isLoading && uiState.note == null) {
                Column(
                    modifier = Modifier.fillMaxSize(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    CircularProgressIndicator()
                }
            } else if (uiState.note == null) {
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(24.dp),
                    verticalArrangement = Arrangement.Center,
                ) {
                    Text(
                        text = uiState.errorMessage ?: stringResource(R.string.notedetail_errorMessage),
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
            } else {
                val note = uiState.note

                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .verticalScroll(scrollState)
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
                        Row(
                            modifier = Modifier.horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
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

                    val primaryColor = MaterialTheme.colorScheme.primary
                    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
                    val baseFontSize = LocalNoteTextSize.current.value

                    val annotatedContent = remember(note.content, primaryColor, highlightColor, baseFontSize) {
                        buildMarkdownAnnotatedString(note.content, primaryColor, highlightColor, baseFontSize, isCompact = false)
                    }

                    Text(
                        text = annotatedContent,
                        style = MaterialTheme.typography.bodyLarge.copy(
                            fontFamily = LocalNoteFontFamily.current,
                            fontWeight = LocalNoteFontWeight.current,
                            fontSize = LocalNoteTextSize.current,
                            lineHeight = LocalNoteTextSize.current * LocalNoteLineSpacing.current
                        ),
                        modifier = Modifier.fillMaxWidth(),
                    )

                    if (uiState.errorMessage != null) {
                        Text(
                            text = uiState.errorMessage,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.padding(top = 16.dp),
                        )
                    }

                    RelationSection(
                        title = stringResource(R.string.notedetail_origins),
                        notes = uiState.origins,
                        onOpenRelatedNote = onOpenRelatedNote,
                    )
                    RelationSection(
                        title = stringResource(R.string.notedetail_previousVersions),
                        notes = uiState.previousVersions,
                        onOpenRelatedNote = onOpenRelatedNote,
                    )
                    RelationSection(
                        title = stringResource(R.string.notedetail_nextVersions),
                        notes = uiState.nextVersions,
                        onOpenRelatedNote = onOpenRelatedNote,
                    )
                    RelationSection(
                        title = stringResource(R.string.notedetail_replies),
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
                    Spacer(modifier = Modifier.height(120.dp))
                }

                HorizontalFloatingToolbar(
                    expanded = true,
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = 24.dp)
                ) {
                    IconButton(onClick = { showDeleteDialog = true }) {
                        Icon(
                            imageVector = Icons.Filled.Delete,
                            contentDescription = stringResource(R.string.delete),
                            modifier = Modifier.size(24.dp)
                        )
                    }

                    IconButton(onClick = {
                        val hasMarkdown = Regex("\\*\\*|(?<!\\*)\\*(?!\\*)|~~|<u>|==|^#{1,4} |^> |^-\\s+\\[[ x]\\]\\s+|^-\\s+|^\\d+\\.\\s+", RegexOption.MULTILINE).containsMatchIn(note.content)
                        if (hasMarkdown) {
                            showCopyDialog = true
                        } else {
                            clipboardManager.setText(AnnotatedString(note.content))
                        }
                    }) {
                        Icon(
                            imageVector = Icons.Filled.ContentCopy,
                            contentDescription = "复制",
                            modifier = Modifier.size(24.dp)
                        )
                    }

                    IconButton(onClick = {
                        val intent = Intent(Intent.ACTION_INSERT).apply {
                            data = CalendarContract.Events.CONTENT_URI
                            putExtra(
                                CalendarContract.Events.TITLE,
                                buildCalendarReminderTitle(note = note, fallback = defaultCalendarTitle),
                            )
                            putExtra(CalendarContract.Events.DESCRIPTION, note.content)
                        }

                        try {
                            context.startActivity(intent)
                        } catch (_: ActivityNotFoundException) {
                            Toast.makeText(context, calendarUnavailableMessage, Toast.LENGTH_SHORT).show()
                        }
                    }) {
                        Icon(
                            imageVector = Icons.Filled.Event,
                            contentDescription = addCalendarReminderLabel,
                            modifier = Modifier.size(24.dp)
                        )
                    }

                    IconButton(onClick = onReply) {
                        Icon(
                            imageVector = Icons.Filled.Reply,
                            contentDescription = stringResource(R.string.reply),
                            modifier = Modifier.size(24.dp)
                        )
                    }

                    IconButton(onClick = onEdit) {
                        Icon(
                            imageVector = Icons.Filled.Edit,
                            contentDescription = stringResource(R.string.edit),
                            modifier = Modifier.size(24.dp)
                        )
                    }
                }
            }
        }
    }
}

private fun buildCalendarReminderTitle(note: Note, fallback: String): String {
    val firstContentLine = note.content
        .lineSequence()
        .map(String::trim)
        .firstOrNull(String::isNotEmpty)
        .orEmpty()

    val sanitized = firstContentLine
        .replace(Regex("^(#{1,4} |> |-\\s+\\[[ x]\\]\\s+|-\\s+|\\d+\\.\\s+)"), "")
        .replace(Regex("\\*\\*\\*|\\*\\*|(?<!\\*)\\*(?!\\*)|~~|<u>|</u>|=="), "")
        .trim()

    return sanitized.take(40).ifEmpty { fallback }
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

    val primaryColor = MaterialTheme.colorScheme.primary
    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
    val baseFontSize = (LocalNoteTextSize.current.value - 2).coerceAtLeast(10f)

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

                    val annotatedContent = remember(note.content, primaryColor, highlightColor, baseFontSize) {
                        buildMarkdownAnnotatedString(note.content, primaryColor, highlightColor, baseFontSize, isCompact = true)
                    }

                    Text(
                        text = annotatedContent,
                        style = MaterialTheme.typography.bodyMedium.copy(
                            fontFamily = LocalNoteFontFamily.current,
                            fontWeight = LocalNoteFontWeight.current,
                            fontSize = baseFontSize.sp,
                            lineHeight = baseFontSize.sp * LocalNoteLineSpacing.current
                        ),
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
