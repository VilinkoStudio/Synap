package com.synap.app.ui.screens

import android.content.ActivityNotFoundException
import android.content.Intent
import android.provider.CalendarContract
import android.widget.Toast
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Alarm
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.LinearScale
import androidx.compose.material.icons.filled.Reply
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.VerticalAlignTop
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.ui.draw.clip
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.ParagraphStyle
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteLineSpacing
import com.synap.app.LocalNoteTextSize
import com.synap.app.R
import com.synap.app.ui.components.ShareExportSheet
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.NoteVersion
import com.synap.app.ui.util.NoteColorUtil
import com.synap.app.ui.util.formatNoteTime
import com.synap.app.ui.viewmodel.DetailUiState
import kotlinx.coroutines.launch
import java.util.concurrent.CancellationException

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

        if (!isCompact) {
            // ========== 核心逻辑：检测连续标题，并动态压缩连续标题的间距 ==========
            val headingMatches = Regex("^(#{1,4} )(.*)", RegexOption.MULTILINE).findAll(visualString).toList()

            for (i in headingMatches.indices) {
                val match = headingMatches[i]
                val level = match.groups[1]!!.value.trim().length
                val scale = 1.8f - (level * 0.15f)
                val headingFontSize = baseFontSize * scale

                addStyle(hiddenSpanStyle, match.groups[1]!!.range.first, match.groups[1]!!.range.last + 1)
                addStyle(SpanStyle(fontWeight = FontWeight.ExtraBold, fontSize = headingFontSize.sp, color = primaryColor), match.groups[2]!!.range.first, match.groups[2]!!.range.last + 1)

                // 将 \n 包含在 ParagraphStyle 的范围内，防止它成为独立的 1.5 倍空行
                val lineEnd = if (match.range.last + 1 < visualString.length && visualString[match.range.last + 1] == '\n') {
                    match.range.last + 2
                } else {
                    match.range.last + 1
                }

                // 检测是否与下一个标题紧紧相连
                val isNextConsecutive = if (i + 1 < headingMatches.size) {
                    val nextMatch = headingMatches[i + 1]
                    val gap = visualString.substring(match.range.last + 1, nextMatch.range.first)
                    gap == "\n" || gap == "\r\n"
                } else false

                // 如果下一行也是标题，则当前行距压缩为极限的 1.0倍；如果是独立/末尾标题，则保持舒适的 1.2倍
                val currentLineHeight = if (isNextConsecutive) headingFontSize * 1.0f else headingFontSize * 1.2f

                addStyle(ParagraphStyle(lineHeight = currentLineHeight.sp), match.range.first, lineEnd)
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
                        // ========== 多行引用对齐：使用透明占位符保证像素级对齐 ==========
                        addStyle(SpanStyle(color = Color.Transparent, fontSize = (baseFontSize * 1.5f).sp, fontWeight = FontWeight.Black), offset, offset + 1)
                        addStyle(hiddenSpanStyle, offset + 1, offset + 2)
                    }
                    addStyle(SpanStyle(color = Color.Gray), offset + 2, offset + lineLength)
                } else {
                    if (inQuote) {
                        inQuote = false
                        // ========== 引用块行距统一调整为 1.2 倍 ==========
                        addStyle(ParagraphStyle(lineHeight = (baseFontSize * 1.2f).sp), quoteStart, offset)
                    }
                }
                offset += lineLength + 1
            }
            if (inQuote) {
                // ========== 兜底：引用块行距统一调整为 1.2 倍 ==========
                addStyle(ParagraphStyle(lineHeight = (baseFontSize * 1.2f).sp), quoteStart, offset - 1)
            }

            Regex("^•( )", RegexOption.MULTILINE).findAll(visualString).forEach { match ->
                addStyle(SpanStyle(color = primaryColor, fontWeight = FontWeight.Bold), match.range.first, match.range.first + 1)
            }
        }

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

        processMatches(Regex("(?<!\\*)\\*(?!\\*)(.*?)(?<!\\*)\\*(?!\\*)"), SpanStyle(fontStyle = FontStyle.Italic))
        processMatches(Regex("~~(.*?)~~"), SpanStyle(textDecoration = TextDecoration.LineThrough))
        processMatches(Regex("<u>(.*?)</u>"), SpanStyle(textDecoration = TextDecoration.Underline))
        processMatches(Regex("==(.*?)=="), SpanStyle(background = highlightColor, color = Color.Black))
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
    onOpenThreadReader: (String) -> Unit,
    onLoadMoreReplies: () -> Unit,
    onRefresh: () -> Unit,
    onExportShare: suspend (List<String>) -> ByteArray,
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

    // 分享 BottomSheet 状态
    var showShareBottomSheet by remember { mutableStateOf(false) }

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

    // 分享 BottomSheet 内容
    if (showShareBottomSheet && uiState.note != null) {
        val noteIdsToShare = remember(uiState) {
            buildList {
                uiState.note?.id?.let(::add)
                addAll(uiState.origins.map { it.id })
                addAll(uiState.previousVersions.map { it.note.id })
                addAll(uiState.nextVersions.map { it.note.id })
                addAll(uiState.replies.map { it.id })
            }.distinct()
        }

        ShareExportSheet(
            noteIds = noteIdsToShare,
            onDismiss = { showShareBottomSheet = false },
            exportShare = onExportShare,
        )
    }

    // ========== 预返回手势核心状态 ==========
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress // 收集系统侧滑进度 (0.0 ~ 1.0)
            }
            onNavigateBack() // 手指松开且达到返回阈值时触发
        } catch (e: CancellationException) {
            backProgress = 0f // 用户取消了侧滑，重置进度
        }
    }

    val noteColor = uiState.note?.let { NoteColorUtil.parseNoteColor(it.tags) }
    val primaryColorForTheme = noteColor ?: MaterialTheme.colorScheme.primary

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            // ========== 应用预返回手势形变 ==========
            .graphicsLayer {
                val scale = 1f - (0.1f * backProgress) // 页面最多缩小到 90%
                scaleX = scale
                scaleY = scale
                translationX = backProgress * 16.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
                shape = RoundedCornerShape(32.dp * backProgress) // 随进度增加圆角
                clip = true
            },
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.notedetail_title)) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = primaryColorForTheme.copy(alpha = 0.1f)
                ),
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
        bottomBar = {
            // ========== 沉浸式固定底部工具栏 ==========
            if (uiState.note != null) {
                Surface(
                    color = primaryColorForTheme.copy(alpha = 0.1f),
                    tonalElevation = 3.dp,
                    shadowElevation = 8.dp,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            // navigationBarsPadding 使小白条颜色与 Surface 一致，且内容不被遮挡
                            .navigationBarsPadding()
                            .padding(horizontal = 8.dp, vertical = 12.dp),
                        horizontalArrangement = Arrangement.SpaceAround,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        val iconTint = MaterialTheme.colorScheme.onSurface

                        IconButton(onClick = { showDeleteDialog = true }) {
                            Icon(
                                imageVector = Icons.Filled.Delete,
                                contentDescription = stringResource(R.string.delete),
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }

                        IconButton(onClick = {
                            val hasMarkdown = Regex("\\*\\*|(?<!\\*)\\*(?!\\*)|~~|<u>|==|^#{1,4} |^> |^-\\s+\\[[ x]\\]\\s+|^-\\s+|^\\d+\\.\\s+", RegexOption.MULTILINE).containsMatchIn(uiState.note.content)
                            if (hasMarkdown) {
                                showCopyDialog = true
                            } else {
                                clipboardManager.setText(AnnotatedString(uiState.note.content))
                            }
                        }) {
                            Icon(
                                imageVector = Icons.Filled.ContentCopy,
                                contentDescription = "复制",
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }

                        IconButton(onClick = {
                            val intent = Intent(Intent.ACTION_INSERT).apply {
                                data = CalendarContract.Events.CONTENT_URI
                                putExtra(
                                    CalendarContract.Events.TITLE,
                                    buildCalendarReminderTitle(note = uiState.note, fallback = defaultCalendarTitle),
                                )
                                putExtra(CalendarContract.Events.DESCRIPTION, uiState.note.content)
                            }

                            try {
                                context.startActivity(intent)
                            } catch (_: ActivityNotFoundException) {
                                Toast.makeText(context, calendarUnavailableMessage, Toast.LENGTH_SHORT).show()
                            }
                        }) {
                            Icon(
                                imageVector = Icons.Filled.Alarm,
                                contentDescription = addCalendarReminderLabel,
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }

                        IconButton(onClick = { showShareBottomSheet = true }) {
                            Icon(
                                imageVector = Icons.Filled.Share,
                                contentDescription = "分享",
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }

                        IconButton(onClick = onReply) {
                            Icon(
                                imageVector = Icons.Filled.Reply,
                                contentDescription = stringResource(R.string.reply),
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }

                        IconButton(onClick = onEdit) {
                            Icon(
                                imageVector = Icons.Filled.Edit,
                                contentDescription = stringResource(R.string.edit),
                                modifier = Modifier.size(24.dp),
                                tint = iconTint
                            )
                        }
                    }
                }
            }
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
                            NoteColorUtil.filterDisplayTags(note.tags).forEach { tag ->
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

                    val primaryColor = noteColor ?: MaterialTheme.colorScheme.primary
                    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
                    val baseFontSize = LocalNoteTextSize.current.value

                    val annotatedContent = remember(note.content, primaryColor, highlightColor, baseFontSize) {
                        buildMarkdownAnnotatedString(note.content, primaryColor, highlightColor, baseFontSize, isCompact = false)
                    }

                    // ========== 允许自由选取文本 ==========
                    SelectionContainer {
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
                    }

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
                        noteColor = noteColor,
                        onOpenRelatedNote = onOpenRelatedNote,
                    )
                    VersionSection(
                        title = stringResource(R.string.notedetail_previousVersions),
                        versions = uiState.previousVersions,
                        noteColor = noteColor,
                        onOpenRelatedNote = onOpenRelatedNote,
                    )
                    VersionSection(
                        title = stringResource(R.string.notedetail_nextVersions),
                        versions = uiState.nextVersions,
                        noteColor = noteColor,
                        onOpenRelatedNote = onOpenRelatedNote,
                    )
                    RelationSection(
                        title = stringResource(R.string.notedetail_replies),
                        notes = uiState.replies,
                        noteColor = noteColor,
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
                    Spacer(modifier = Modifier.height(32.dp))
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
    noteColor: Color?,
    onOpenRelatedNote: (String) -> Unit,
) {
    if (notes.isEmpty()) {
        return
    }

    val primaryColor = noteColor ?: MaterialTheme.colorScheme.primary

    Text(
        text = title,
        style = MaterialTheme.typography.titleMedium,
        color = primaryColor,
        modifier = Modifier.padding(top = 24.dp, bottom = 12.dp),
    )

    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
    val baseFontSize = (LocalNoteTextSize.current.value - 2).coerceAtLeast(10f)
    val cardBackgroundColor = primaryColor.copy(alpha = 0.08f)

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        notes.forEach { note ->
            Surface(
                shape = MaterialTheme.shapes.medium,
                color = cardBackgroundColor,
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
                        maxLines = 6,
                        overflow = TextOverflow.Ellipsis,
                        style = MaterialTheme.typography.bodyMedium.copy(
                            fontFamily = LocalNoteFontFamily.current,
                            fontWeight = LocalNoteFontWeight.current,
                            fontSize = baseFontSize.sp,
                            lineHeight = baseFontSize.sp * LocalNoteLineSpacing.current
                        ),
                    )
                    val relationDisplayTags = NoteColorUtil.filterDisplayTags(note.tags)
                    if (relationDisplayTags.isNotEmpty()) {
                        Text(
                            text = relationDisplayTags.joinToString(" · "),
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

@Composable
private fun VersionSection(
    title: String,
    versions: List<NoteVersion>,
    noteColor: Color?,
    onOpenRelatedNote: (String) -> Unit,
) {
    if (versions.isEmpty()) {
        return
    }

    val primaryColor = noteColor ?: MaterialTheme.colorScheme.primary

    Text(
        text = title,
        style = MaterialTheme.typography.titleMedium,
        color = primaryColor,
        modifier = Modifier.padding(top = 24.dp, bottom = 12.dp),
    )

    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
    val baseFontSize = (LocalNoteTextSize.current.value - 2).coerceAtLeast(10f)
    val cardBackgroundColor = primaryColor.copy(alpha = 0.08f)

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        versions.forEach { version ->
            VersionCard(
                version = version,
                primaryColor = primaryColor,
                highlightColor = highlightColor,
                baseFontSize = baseFontSize,
                cardBackgroundColor = cardBackgroundColor,
                onOpenRelatedNote = onOpenRelatedNote,
            )
        }
    }
}

@Composable
private fun VersionCard(
    version: NoteVersion,
    primaryColor: Color,
    highlightColor: Color,
    baseFontSize: Float,
    cardBackgroundColor: Color,
    onOpenRelatedNote: (String) -> Unit,
) {
    val note = version.note

    Surface(
        shape = MaterialTheme.shapes.medium,
        color = cardBackgroundColor,
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onOpenRelatedNote(note.id) },
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            val annotatedContent = remember(note.content, primaryColor, highlightColor, baseFontSize) {
                buildMarkdownAnnotatedString(
                    note.content,
                    primaryColor,
                    highlightColor,
                    baseFontSize,
                    isCompact = true,
                )
            }

            Text(
                text = annotatedContent,
                maxLines = 6,
                overflow = TextOverflow.Ellipsis,
                style = MaterialTheme.typography.bodyMedium.copy(
                    fontFamily = LocalNoteFontFamily.current,
                    fontWeight = LocalNoteFontWeight.current,
                    fontSize = baseFontSize.sp,
                    lineHeight = baseFontSize.sp * LocalNoteLineSpacing.current,
                ),
            )

            val versionDisplayTags = NoteColorUtil.filterDisplayTags(note.tags)
            if (versionDisplayTags.isNotEmpty()) {
                Text(
                    text = versionDisplayTags.joinToString(" · "),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 6.dp),
                )
            }

            val hasTagDiff = version.addedTags.isNotEmpty() || version.removedTags.isNotEmpty()
            val hasTextDiff = version.diffStats.insertedChars > 0u || version.diffStats.deletedChars > 0u
            if (hasTagDiff || hasTextDiff) {
                Column(
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.padding(top = 10.dp),
                ) {
                    if (hasTagDiff) {
                        // ========== 简化差异 Tag 展示：去除了外层包裹，只保留一行内容 ==========
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.horizontalScroll(rememberScrollState()),
                        ) {
                            version.addedTags.forEach { tag ->
                                DiffTagChip(
                                    text = "+#$tag",
                                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                )
                            }
                            version.removedTags.forEach { tag ->
                                DiffTagChip(
                                    text = "-#$tag",
                                    containerColor = MaterialTheme.colorScheme.errorContainer,
                                    contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                )
                            }
                        }
                    }

                    if (hasTextDiff) {
                        val diffMetrics = buildList {
                            if (version.diffStats.insertedChars > 0u) {
                                add(
                                    Triple(
                                        "+${version.diffStats.insertedChars}字符",
                                        MaterialTheme.colorScheme.primaryContainer,
                                        MaterialTheme.colorScheme.onPrimaryContainer,
                                    ),
                                )
                            }
                            if (version.diffStats.deletedChars > 0u) {
                                add(
                                    Triple(
                                        "-${version.diffStats.deletedChars}字符",
                                        MaterialTheme.colorScheme.errorContainer,
                                        MaterialTheme.colorScheme.onErrorContainer,
                                    ),
                                )
                            }
                            if (version.diffStats.insertedLines > 0u) {
                                add(
                                    Triple(
                                        "+${version.diffStats.insertedLines}行",
                                        MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.82f),
                                        MaterialTheme.colorScheme.onPrimaryContainer,
                                    ),
                                )
                            }
                            if (version.diffStats.deletedLines > 0u) {
                                add(
                                    Triple(
                                        "-${version.diffStats.deletedLines}行",
                                        MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.82f),
                                        MaterialTheme.colorScheme.onErrorContainer,
                                    ),
                                )
                            }
                        }

                        Row(
                            verticalAlignment = Alignment.Top,
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(10.dp))
                                .background(MaterialTheme.colorScheme.surface)
                                .padding(horizontal = 10.dp, vertical = 8.dp),
                        ) {
                            diffMetrics.forEachIndexed { index, (text, containerColor, contentColor) ->
                                if (index > 0) {
                                    Spacer(modifier = Modifier.width(8.dp))
                                }
                                DiffMetricPill(
                                    text = text,
                                    containerColor = containerColor,
                                    contentColor = contentColor,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun DiffTagChip(
    text: String,
    containerColor: Color,
    contentColor: Color,
) {
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = containerColor,
    ) {
        Text(
            text = text,
            color = contentColor,
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
        )
    }
}

@Composable
private fun DiffMetricPill(
    text: String,
    containerColor: Color,
    contentColor: Color,
) {
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = containerColor,
    ) {
        Text(
            text = text,
            color = contentColor,
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 5.dp),
        )
    }
}

// ==================== 二维码生成工具 ====================
fun generateQRCodeBitmap(text: String, size: Int = 512, primaryColor: Int = android.graphics.Color.BLACK, backgroundColor: Int = android.graphics.Color.WHITE): android.graphics.Bitmap? {
    if (text.isEmpty()) return null
    return try {
        val hints = mapOf(
            com.google.zxing.EncodeHintType.CHARACTER_SET to "UTF-8",
            com.google.zxing.EncodeHintType.MARGIN to 1 // 缩小留白
        )
        val bitMatrix = com.google.zxing.MultiFormatWriter().encode(
            text,
            com.google.zxing.BarcodeFormat.QR_CODE,
            size,
            size,
            hints
        )
        val width = bitMatrix.width
        val height = bitMatrix.height
        val pixels = IntArray(width * height)
        for (y in 0 until height) {
            val offset = y * width
            for (x in 0 until width) {
                pixels[offset + x] = if (bitMatrix[x, y]) primaryColor else backgroundColor
            }
        }
        val bitmap = android.graphics.Bitmap.createBitmap(width, height, android.graphics.Bitmap.Config.ARGB_8888)
        bitmap.setPixels(pixels, 0, width, 0, 0, width, height)
        bitmap
    } catch (e: Exception) {
        e.printStackTrace()
        null
    }
}