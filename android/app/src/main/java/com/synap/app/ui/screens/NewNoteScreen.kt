package com.synap.app.ui.screens

import android.content.Context
import android.widget.Toast
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.absoluteOffset
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.CheckBox
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.FormatBold
import androidx.compose.material.icons.filled.FormatColorText
import androidx.compose.material.icons.filled.FormatItalic
import androidx.compose.material.icons.filled.FormatListBulleted
import androidx.compose.material.icons.filled.FormatQuote
import androidx.compose.material.icons.filled.FormatStrikethrough
import androidx.compose.material.icons.filled.FormatUnderlined
import androidx.compose.material.icons.filled.Inventory2
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.InputChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.ParagraphStyle
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteLineSpacing
import com.synap.app.LocalNoteTextSize
import com.synap.app.R
import com.synap.app.ui.util.NoteColorUtil
import com.synap.app.ui.viewmodel.EditorMode
import com.synap.app.ui.viewmodel.EditorUiState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay

// ==================== 独立私有颜色解析 ====================
private fun parseColorLocal(tag: String): Color? {
    val t = tag.trim().lowercase()
    if (t.startsWith("#")) {
        return try { Color(android.graphics.Color.parseColor(t)) } catch (e: Exception) { null }
    }
    val rgbRegex = Regex("""rgb\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*\)""")
    val match = rgbRegex.matchEntire(t)
    if (match != null) {
        val r = match.groupValues[1].toIntOrNull()?.coerceIn(0, 255) ?: 0
        val g = match.groupValues[2].toIntOrNull()?.coerceIn(0, 255) ?: 0
        val b = match.groupValues[3].toIntOrNull()?.coerceIn(0, 255) ?: 0
        return Color(r, g, b)
    }
    return try { Color(android.graphics.Color.parseColor(t)) } catch (e: Exception) { null }
}

data class CheckboxInfo(val range: IntRange, val isChecked: Boolean, val rect: Rect)

data class CustomColor(val hue: Float, val name: String)

private fun saveCustomColor(context: Context, color: CustomColor) {
    val prefs = context.getSharedPreferences("custom_colors", Context.MODE_PRIVATE)
    val existing = prefs.getString("colors", "[]") ?: "[]"
    val arr = org.json.JSONArray(existing)
    val obj = org.json.JSONObject().apply {
        put("hue", color.hue.toDouble())
        put("name", color.name)
    }
    arr.put(obj)
    prefs.edit().putString("colors", arr.toString()).apply()
}

private fun loadCustomColors(context: Context): List<CustomColor> {
    val prefs = context.getSharedPreferences("custom_colors", Context.MODE_PRIVATE)
    val existing = prefs.getString("colors", "[]") ?: "[]"
    val arr = org.json.JSONArray(existing)
    return (0 until arr.length()).map { i ->
        val obj = arr.getJSONObject(i)
        CustomColor(
            hue = obj.getDouble("hue").toFloat(),
            name = obj.getString("name")
        )
    }
}

private fun deleteCustomColor(context: Context, index: Int) {
    val prefs = context.getSharedPreferences("custom_colors", Context.MODE_PRIVATE)
    val existing = prefs.getString("colors", "[]") ?: "[]"
    val arr = org.json.JSONArray(existing)
    val newArr = org.json.JSONArray()
    (0 until arr.length()).forEach { i ->
        if (i != index) newArr.put(arr.getJSONObject(i))
    }
    prefs.edit().putString("colors", newArr.toString()).apply()
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class, ExperimentalSharedTransitionApi::class)
@Composable
fun NewNoteScreen(
    uiState: EditorUiState,
    onNavigateBack: () -> Unit,
    onNavigateToHome: () -> Unit,
    onContentChange: (String) -> Unit,
    onAddTag: (String) -> Unit,
    onRemoveTag: (Int) -> Unit,
    onNoteColorHueChange: (Float?) -> Unit,
    onSave: () -> Unit,
    onNavigateToDrafts: () -> Unit,
    draftCount: Int,
    hasUnsavedChanges: Boolean,
    onSaveDraft: () -> Unit,
    onDiscardDraft: () -> Unit,
    isContentMatchingLatestDraft: () -> Boolean,
    onMarkDraftAsRead: (String) -> Unit,
    onRefreshDraftCount: () -> Unit,
    sharedTransitionScope: SharedTransitionScope? = null,
    animatedVisibilityScope: AnimatedVisibilityScope? = null,
) {
    var tagInputText by remember { mutableStateOf("") }
    var isTagInputVisible by remember { mutableStateOf(false) }
    val tagFocusRequester = remember { FocusRequester() }
    val keyboardController = LocalSoftwareKeyboardController.current

    var showColorDialog by remember { mutableStateOf(false) }
    var localColorHue by remember { mutableFloatStateOf(uiState.noteColorHue ?: 210f) }
    var showAddPresetDialog by remember { mutableStateOf(false) }
    val context = LocalContext.current
    var customColors by remember { mutableStateOf(loadCustomColors(context)) }

    // 检查是否有需要恢复的草稿（进程意外终止）
    val draftStore = remember { com.synap.app.data.service.DraftStore(context) }
    var recoveryDraft by remember { mutableStateOf<com.synap.app.data.service.DraftRecord?>(null) }

    LaunchedEffect(Unit) {
        val latestDraft = draftStore.getLatestDraft()
        if (latestDraft != null) {
            if (isContentMatchingLatestDraft()) {
                // 内容相同，标记为已读，不显示弹窗
                onMarkDraftAsRead(latestDraft.id)
            } else if (latestDraft.status == "pending") {
                // 有pending状态的笔记，显示恢复弹窗
                recoveryDraft = latestDraft
            } else if (latestDraft.reason == "auto" && latestDraft.status != "read") {
                // 内容不同，将最新草稿状态改为pending，显示恢复弹窗
                draftStore.updateStatus(latestDraft.id, "pending")
                recoveryDraft = latestDraft.copy(status = "pending")
            }
        }
    }

    LaunchedEffect(uiState.noteColorHue) {
        if (uiState.noteColorHue != null) localColorHue = uiState.noteColorHue!!
    }

    var textFieldValue by remember {
        mutableStateOf(TextFieldValue(text = uiState.content, selection = TextRange(uiState.content.length)))
    }

    LaunchedEffect(uiState.content) {
        if (uiState.content != textFieldValue.text) {
            textFieldValue = textFieldValue.copy(text = uiState.content)
        }
    }

    val applyStyle: (String, String) -> Unit = { prefix, suffix ->
        val start = textFieldValue.selection.min
        val end = textFieldValue.selection.max
        val text = textFieldValue.text
        val newText = text.substring(0, start) + prefix + text.substring(start, end) + suffix + text.substring(end)
        val newSelection = TextRange(start + prefix.length, end + prefix.length)
        textFieldValue = textFieldValue.copy(text = newText, selection = newSelection)
        onContentChange(newText)
    }

    // 智能行首格式替换与有序列表打断逻辑
    val applyLinePrefix: (String) -> Unit = { newPrefix ->
        val text = textFieldValue.text
        val selection = textFieldValue.selection
        var lineStart = selection.min
        while (lineStart > 0 && text[lineStart - 1] != '\n') lineStart--
        var lineEnd = selection.max
        while (lineEnd < text.length && text[lineEnd] != '\n') lineEnd++

        val currentLine = text.substring(lineStart, lineEnd)
        val regex = Regex("^(#{1,6}\\s|>\\s|-\\s\\[[ xX]\\]\\s|-\\s|\\d+\\.\\s)")
        val match = regex.find(currentLine)

        if (match != null) {
            val existingPrefix = match.value
            // 如果已存在且相同 -> 取消格式
            if (existingPrefix == newPrefix || (Regex("^\\d+\\.\\s").matches(existingPrefix) && Regex("^\\d+\\.\\s").matches(newPrefix))) {
                val newText = text.substring(0, lineStart) + currentLine.substring(existingPrefix.length) + text.substring(lineEnd)
                val diff = newText.length - text.length
                textFieldValue = textFieldValue.copy(text = newText, selection = TextRange((selection.start + diff).coerceAtLeast(0)))
                onContentChange(newText)
            } else {
                // 如果存在但不同 -> 替换格式
                val newText = text.substring(0, lineStart) + newPrefix + currentLine.substring(existingPrefix.length) + text.substring(lineEnd)
                val diff = newPrefix.length - existingPrefix.length
                textFieldValue = textFieldValue.copy(text = newText, selection = TextRange((selection.start + diff).coerceAtLeast(0)))
                onContentChange(newText)
            }
        } else {
            // 没有格式，直接应用 (特殊处理有序列表的自增)
            val prefixToApply = if (newPrefix == "1. ") {
                var prevNum = 0
                if (lineStart > 0) {
                    var prevLineStart = lineStart - 1
                    while (prevLineStart > 0 && text[prevLineStart - 1] != '\n') prevLineStart--
                    val prevLine = text.substring(prevLineStart, lineStart - 1)
                    val prevMatch = Regex("^(\\d+)\\.\\s").find(prevLine)
                    if (prevMatch != null) prevNum = prevMatch.groups[1]!!.value.toIntOrNull() ?: 0
                }
                "${prevNum + 1}. "
            } else newPrefix

            val newText = text.substring(0, lineStart) + prefixToApply + currentLine + text.substring(lineEnd)
            textFieldValue = textFieldValue.copy(text = newText, selection = TextRange(selection.start + prefixToApply.length))
            onContentChange(newText)
        }
    }

    fun hideKeyboardAndNavigate(action: () -> Unit) {
        keyboardController?.hide()
        action()
    }

    var showUnsavedDialog by remember { mutableStateOf(false) }
    var showBackDialog by remember { mutableStateOf(false) }

    var backProgress by remember { mutableFloatStateOf(0f) }

    fun handleBackNavigation() {
        if (hasUnsavedChanges) {
            backProgress = 0f // 归位预返回手势动画
            showBackDialog = true
        } else {
            onNavigateBack()
        }
    }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent -> backProgress = backEvent.progress }
            hideKeyboardAndNavigate { handleBackNavigation() }
        } catch (e: CancellationException) {
            backProgress = 0f
        }
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            .graphicsLayer {
                translationX = backProgress * 64.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            }
            .let {
                if (uiState.mode == EditorMode.Create && sharedTransitionScope != null && animatedVisibilityScope != null) {
                    with(sharedTransitionScope) {
                        it.sharedBounds(
                            sharedContentState = rememberSharedContentState(key = "fab_to_new_note"),
                            animatedVisibilityScope = animatedVisibilityScope
                        )
                    }
                } else it
            },
        topBar = {
            TopAppBar(
                title = { Text(stringResource(when (uiState.mode) {
                    is EditorMode.Create -> R.string.edit_title_creat
                    is EditorMode.Reply -> R.string.edit_title_reply
                    is EditorMode.Edit -> R.string.edit_title_edit
                })) },
                navigationIcon = { IconButton(onClick = { hideKeyboardAndNavigate { handleBackNavigation() } }) { Icon(Icons.Filled.ArrowBack, "返回") } },
                actions = {
                    val isTabletTopBar = LocalConfiguration.current.screenWidthDp >= 700
                    if (!isTabletTopBar) {
                        val currentColor = uiState.noteColorHue?.let { NoteColorUtil.hueToColor(it) }
                        TextButton(
                            onClick = { showColorDialog = true },
                            colors = ButtonDefaults.textButtonColors(
                                containerColor = currentColor?.copy(alpha = 0.15f)
                                    ?: MaterialTheme.colorScheme.primary.copy(alpha = 0.1f)
                            )
                        ) {
                            if (currentColor != null) {
                                Box(modifier = Modifier.size(10.dp).background(currentColor, CircleShape))
                                Spacer(modifier = Modifier.width(6.dp))
                            }
                            Text("笔记颜色", color = currentColor ?: MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.labelMedium)
                        }
                    }
                    // Draft box icon
                    if (draftCount > 0) {
                        BadgedBox(
                            badge = { Badge { Text("$draftCount") } }
                        ) {
                            IconButton(onClick = { hideKeyboardAndNavigate { onNavigateToDrafts() } }) {
                                Icon(Icons.Filled.Inventory2, "草稿箱")
                            }
                        }
                    } else {
                        IconButton(onClick = { hideKeyboardAndNavigate { onNavigateToDrafts() } }) {
                            Icon(Icons.Filled.Inventory2, "草稿箱")
                        }
                    }
                    IconButton(onClick = { hideKeyboardAndNavigate { onSave() } }, enabled = !uiState.isSaving && !uiState.isLoading) {
                        if (uiState.isSaving) CircularProgressIndicator(modifier = Modifier.size(24.dp)) else Icon(Icons.Filled.Check, "保存")
                    }
                },
            )
        },
        bottomBar = {
            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .imePadding(),
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 3.dp,
                shadowElevation = 8.dp
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .navigationBarsPadding()
                        .padding(horizontal = 8.dp, vertical = 8.dp)
                        .horizontalScroll(rememberScrollState()),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    val iconColor = MaterialTheme.colorScheme.onSurface
                    val textStyle = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold, color = iconColor)

                    IconButton(onClick = { applyLinePrefix("- [ ] ") }) { Icon(Icons.Filled.CheckBox, null, tint = iconColor) }
                    IconButton(onClick = { applyLinePrefix("- ") }) { Icon(Icons.Filled.FormatListBulleted, null, tint = iconColor) }
                    IconButton(onClick = { applyLinePrefix("1. ") }) { Text("1.", style = textStyle) }
                    IconButton(onClick = { applyLinePrefix("# ") }) { Text("H1", style = textStyle) }
                    IconButton(onClick = { applyLinePrefix("## ") }) { Text("H2", style = textStyle) }
                    IconButton(onClick = { applyLinePrefix("> ") }) { Icon(Icons.Filled.FormatQuote, null, tint = iconColor) }
                    IconButton(onClick = { applyStyle("**", "**") }) { Icon(Icons.Filled.FormatBold, null, tint = iconColor) }
                    IconButton(onClick = { applyStyle("*", "*") }) { Icon(Icons.Filled.FormatItalic, null, tint = iconColor) }
                    IconButton(onClick = { applyStyle("<u>", "</u>") }) { Icon(Icons.Filled.FormatUnderlined, null, tint = iconColor) }
                    IconButton(onClick = { applyStyle("~~", "~~") }) { Icon(Icons.Filled.FormatStrikethrough, null, tint = iconColor) }
                    IconButton(onClick = { applyStyle("==", "==") }) { Icon(Icons.Filled.FormatColorText, null, tint = iconColor) }
                }
            }
        }
    ) { innerPadding ->

        BoxWithConstraints(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .consumeWindowInsets(innerPadding)
        ) {
            val isTablet = maxWidth >= 700.dp

            val TagSectionContent: @Composable () -> Unit = {
                Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)) {
                    // 大屏幕：标签栏顶部直接放颜色选择器
                    if (isTablet) {
                        Text("笔记颜色", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = 8.dp, bottom = 4.dp))
                        NoteColorPickerContent(
                            currentHue = uiState.noteColorHue,
                            localHue = localColorHue,
                            onLocalHueChange = { localColorHue = it },
                            onColorChange = { onNoteColorHueChange(it) },
                            onClear = { onNoteColorHueChange(null) },
                            customColors = customColors,
                            onAddPreset = { showAddPresetDialog = true },
                            onDeleteCustomColor = { index ->
                                deleteCustomColor(context, index)
                                customColors = loadCustomColors(context)
                            }
                        )
                        HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                    }

                    if (isTagInputVisible) {
                        var hasGainedFocus by remember { mutableStateOf(false) }
                        LaunchedEffect(Unit) { try { tagFocusRequester.requestFocus() } catch (e: Exception) {} }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            OutlinedTextField(
                                value = tagInputText,
                                onValueChange = { tagInputText = it },
                                placeholder = { Text("输入标签") },
                                modifier = Modifier
                                    .weight(1f)
                                    .height(56.dp)
                                    .focusRequester(tagFocusRequester)
                                    .onFocusChanged { focusState ->
                                        if (focusState.isFocused) {
                                            hasGainedFocus = true
                                        } else if (hasGainedFocus && tagInputText.isBlank()) {
                                            isTagInputVisible = false
                                        }
                                    },
                                singleLine = true,
                                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                                keyboardActions = KeyboardActions(onDone = {
                                    if (tagInputText.isNotBlank()) {
                                        onAddTag(tagInputText.trim())
                                        tagInputText = ""
                                        isTagInputVisible = false
                                    } else {
                                        isTagInputVisible = false
                                    }
                                })
                            )

                            IconButton(
                                onClick = {
                                    if (tagInputText.isNotBlank()) {
                                        onAddTag(tagInputText.trim())
                                        tagInputText = ""
                                    }
                                    isTagInputVisible = false
                                }
                            ) {
                                Icon(Icons.Filled.Check, contentDescription = "确认添加", tint = MaterialTheme.colorScheme.primary)
                            }
                        }
                    } else {
                        val chipsContent: @Composable () -> Unit = {
                            uiState.tags.forEachIndexed { i, tag ->
                                val parsedColor = parseColorLocal(tag)
                                InputChip(
                                    selected = true,
                                    onClick = {},
                                    label = { Text(tag) },
                                    leadingIcon = parsedColor?.let { color ->
                                        { Box(modifier = Modifier.size(8.dp).background(color, CircleShape)) }
                                    },
                                    trailingIcon = { Icon(Icons.Filled.Close, null, Modifier.size(InputChipDefaults.AvatarSize).clickable { onRemoveTag(i) }) }
                                )
                            }
                            InputChip(selected = false, onClick = { isTagInputVisible = true }, label = { Text("添加标签") }, trailingIcon = { Icon(Icons.Filled.Add, null, Modifier.size(16.dp)) })
                        }

                        if (isTablet) {
                            FlowRow(
                                modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                chipsContent()
                            }
                        } else {
                            Row(
                                modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                chipsContent()
                            }
                        }
                    }

                    if (uiState.recommendedTags.isNotEmpty() || uiState.isRecommendingTags) {
                        val recommendedContent: @Composable () -> Unit = {
                            Text("推荐标签：", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(vertical = 4.dp))
                            if (uiState.isRecommendingTags) {
                                CircularProgressIndicator(modifier = Modifier.size(14.dp).padding(start = 4.dp, top = 4.dp), strokeWidth = 2.dp)
                            } else {
                                uiState.recommendedTags.forEach { tag ->
                                    Text(
                                        text = "#$tag",
                                        style = MaterialTheme.typography.labelLarge,
                                        color = MaterialTheme.colorScheme.primary,
                                        modifier = Modifier.clip(RoundedCornerShape(4.dp)).clickable { onAddTag(tag) }.padding(horizontal = 4.dp, vertical = 2.dp)
                                    )
                                }
                            }
                        }

                        if (isTablet) {
                            FlowRow(
                                modifier = Modifier.fillMaxWidth().padding(top = 4.dp, bottom = 8.dp),
                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                                verticalArrangement = Arrangement.spacedBy(4.dp)
                            ) {
                                recommendedContent()
                            }
                        } else {
                            Row(
                                modifier = Modifier.fillMaxWidth().padding(top = 4.dp, bottom = 8.dp).horizontalScroll(rememberScrollState()),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                recommendedContent()
                            }
                        }
                    }

                    if (!isTablet) {
                        HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                    }
                }
            }

            val EditorSectionContent: @Composable () -> Unit = {
                Box(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
                    if (uiState.isLoading) {
                        CircularProgressIndicator(modifier = Modifier.align(Alignment.Center))
                    } else {
                        val primaryColor = MaterialTheme.colorScheme.primary
                        val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
                        val baseFontSizeUnit = LocalNoteTextSize.current
                        val baseFontSize = baseFontSizeUnit.value

                        var textLayoutResult by remember { mutableStateOf<TextLayoutResult?>(null) }
                        val focusRequester = remember { FocusRequester() }
                        val textScrollState = rememberScrollState()

                        var checkboxRects by remember { mutableStateOf<List<CheckboxInfo>>(emptyList()) }
                        val checkboxRegexPattern = "^-\\s+\\[([ xX])\\]\\s?"
                        val currentSelection by rememberUpdatedState(textFieldValue.selection)

                        // ========== 修复 1：引入 BringIntoViewRequester 手动控制光标滚动 ==========
                        val bringIntoViewRequester = remember { BringIntoViewRequester() }
                        val density = LocalDensity.current

                        LaunchedEffect(Unit) {
                            delay(300)
                            focusRequester.requestFocus()
                            keyboardController?.show()
                        }

                        // ========== 修复 2：监听光标和文字变化，计算出足够安全的区域要求组件滚动 ==========
                        LaunchedEffect(textFieldValue.selection.start, textFieldValue.text.length) {
                            delay(50) // 给点时间等待布局刷新和键盘弹出动画
                            textLayoutResult?.let { layoutResult ->
                                try {
                                    val offset = textFieldValue.selection.start.coerceIn(0, textFieldValue.text.length)
                                    val cursorRect = layoutResult.getCursorRect(offset)
                                    // 核心逻辑：在光标的下方加上 120dp 的“虚拟要求可视空间”
                                    // 这将强迫 ScrollState 把整个区域往上推，让光标完美待在键盘和候选词的上方
                                    val paddedRect = Rect(
                                        left = cursorRect.left,
                                        top = cursorRect.top,
                                        right = cursorRect.right,
                                        bottom = cursorRect.bottom + with(density) { 120.dp.toPx() }
                                    )
                                    bringIntoViewRequester.bringIntoView(paddedRect)
                                } catch (e: Exception) {
                                    // 忽略快速输入时可能发生的短暂坐标越界
                                }
                            }
                        }

                        Column(modifier = Modifier.fillMaxSize().verticalScroll(textScrollState)) {
                            Box(modifier = Modifier.fillMaxWidth()) {
                                if (textFieldValue.text.isEmpty()) {
                                    Text(
                                        text = "开始记录你的灵感...",
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                                        style = MaterialTheme.typography.bodyLarge.copy(
                                            fontFamily = LocalNoteFontFamily.current,
                                            fontSize = baseFontSize.sp
                                        ),
                                    )
                                }

                                BasicTextField(
                                    value = textFieldValue,
                                    onValueChange = { newValue ->
                                        val oldText = textFieldValue.text
                                        val newText = newValue.text
                                        var finalValue = newValue

                                        if (newText.length == oldText.length + 1 && newValue.selection.start > 0 && newText[newValue.selection.start - 1] == '\n') {
                                            val beforeNewline = newText.substring(0, newValue.selection.start - 1)
                                            val lastLineStart = beforeNewline.lastIndexOf('\n').let { if (it == -1) 0 else it + 1 }
                                            val lastLine = beforeNewline.substring(lastLineStart)

                                            var handled = false

                                            val orderedMatch = Regex("^(\\d+)\\.\\s(.*)").find(lastLine)
                                            if (orderedMatch != null) {
                                                handled = true
                                                if (orderedMatch.groups[2]!!.value.isEmpty()) {
                                                    val removeLen = lastLine.length
                                                    val resultingText = newText.removeRange(newValue.selection.start - 1 - removeLen, newValue.selection.start - 1)
                                                    finalValue = TextFieldValue(resultingText, TextRange(newValue.selection.start - removeLen))
                                                } else {
                                                    val num = orderedMatch.groups[1]!!.value.toIntOrNull()
                                                    if (num != null) {
                                                        val insert = "${num + 1}. "
                                                        val resultingText = newText.substring(0, newValue.selection.start) + insert + newText.substring(newValue.selection.start)
                                                        finalValue = TextFieldValue(resultingText, TextRange(newValue.selection.start + insert.length))
                                                    }
                                                }
                                            }

                                            if (!handled) {
                                                val bulletMatch = Regex("^(-( \\[[ xX]\\])?)\\s(.*)").find(lastLine)
                                                if (bulletMatch != null) {
                                                    if (bulletMatch.groups[3]!!.value.isEmpty()) {
                                                        val removeLen = lastLine.length
                                                        val resultingText = newText.removeRange(newValue.selection.start - 1 - removeLen, newValue.selection.start - 1)
                                                        finalValue = TextFieldValue(resultingText, TextRange(newValue.selection.start - removeLen))
                                                    } else {
                                                        val isCheckbox = bulletMatch.groups[1]!!.value.contains("[")
                                                        val insert = if (isCheckbox) "- [ ] " else "- "
                                                        val resultingText = newText.substring(0, newValue.selection.start) + insert + newText.substring(newValue.selection.start)
                                                        finalValue = TextFieldValue(resultingText, TextRange(newValue.selection.start + insert.length))
            }
        }
    }
}

                                        if (newText.length == oldText.length - 1 && newValue.selection.start == textFieldValue.selection.start - 1) {
                                            val deletedIndex = newValue.selection.start
                                            Regex(checkboxRegexPattern, RegexOption.MULTILINE).findAll(oldText).forEach { match ->
                                                if (deletedIndex in match.range) {
                                                    val resultingText = oldText.removeRange(match.range)
                                                    finalValue = TextFieldValue(resultingText, TextRange(match.range.first))
                                                }
                                            }
                                        }

                                        textFieldValue = finalValue
                                        onContentChange(finalValue.text)
                                    },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .heightIn(min = 300.dp)
                                        .focusRequester(focusRequester)
                                        // 绑定请求器，允许外部强制让该组件内部的某一个区域显示在屏幕上
                                        .bringIntoViewRequester(bringIntoViewRequester),
                                    textStyle = MaterialTheme.typography.bodyLarge.copy(
                                        fontFamily = LocalNoteFontFamily.current,
                                        fontWeight = LocalNoteFontWeight.current,
                                        fontSize = baseFontSize.sp,
                                        lineHeight = (baseFontSize * LocalNoteLineSpacing.current).sp,
                                        color = MaterialTheme.colorScheme.onSurface
                                    ),
                                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                                    onTextLayout = { result ->
                                        textLayoutResult = result
                                        val newRects = mutableListOf<CheckboxInfo>()
                                        Regex(checkboxRegexPattern, RegexOption.MULTILINE).findAll(textFieldValue.text).forEach { match ->
                                            val rectStart = result.getBoundingBox(match.range.first)
                                            newRects.add(CheckboxInfo(match.range, match.groups[1]!!.value.lowercase() == "x", rectStart))
                                        }
                                        checkboxRects = newRects
                                    },
                                    visualTransformation = remember(primaryColor, highlightColor, baseFontSize) {
                                        VisualTransformation { annotatedString ->
                                            val builder = AnnotatedString.Builder(annotatedString.text)
                                            val text = annotatedString.text

                                            val hiddenSpanStyle = SpanStyle(color = Color.Transparent, fontSize = 0.1f.sp)
                                            val transparentSpanStyle = SpanStyle(color = Color.Transparent)
                                            val revealedSpanStyle = SpanStyle(color = primaryColor.copy(alpha = 0.5f))

                                            fun applySyntax(range: IntRange, useHidden: Boolean = true) {
                                                val selMin = currentSelection.min
                                                val selMax = currentSelection.max
                                                val isCursorNear = (selMin <= range.last + 1 && selMax >= range.first)

                                                if (isCursorNear) {
                                                    builder.addStyle(revealedSpanStyle, range.first, range.last + 1)
                                                } else {
                                                    builder.addStyle(if (useHidden) hiddenSpanStyle else transparentSpanStyle, range.first, range.last + 1)
                                                }
                                            }

                                            val headingMatches = Regex("^(#{1,4} )(.*)", RegexOption.MULTILINE).findAll(text).toList()
                                            for (i in headingMatches.indices) {
                                                val match = headingMatches[i]
                                                val syntax = match.groups[1]!!
                                                val content = match.groups[2]!!
                                                val level = syntax.value.trim().length
                                                val scale = 1.8f - (level * 0.15f)
                                                val headingFontSize = baseFontSize * scale

                                                builder.addStyle(SpanStyle(fontWeight = FontWeight.ExtraBold, fontSize = headingFontSize.sp, color = primaryColor), content.range.first, content.range.last + 1)
                                                applySyntax(syntax.range)

                                                val lineEnd = if (match.range.last + 1 < text.length && text[match.range.last + 1] == '\n') match.range.last + 2 else match.range.last + 1
                                                val isNextConsecutive = if (i + 1 < headingMatches.size) {
                                                    val nextMatch = headingMatches[i + 1]
                                                    val gap = text.substring(match.range.last + 1, nextMatch.range.first)
                                                    gap == "\n" || gap == "\r\n"
                                                } else false

                                                val currentLineHeight = if (isNextConsecutive) headingFontSize * 1.0f else headingFontSize * 1.2f
                                                builder.addStyle(ParagraphStyle(lineHeight = currentLineHeight.sp), match.range.first, lineEnd)
                                            }

                                            Regex(checkboxRegexPattern, RegexOption.MULTILINE).findAll(text).forEach { match ->
                                                applySyntax(match.range, useHidden = false)
                                                val isChecked = match.groups[1]!!.value.lowercase() == "x"
                                                if (isChecked) {
                                                    val lineEnd = text.indexOf('\n', match.range.last).takeIf { it != -1 } ?: text.length
                                                    builder.addStyle(SpanStyle(color = Color.Gray, textDecoration = TextDecoration.LineThrough), match.range.last + 1, lineEnd)
                                                }
                                            }

                                            Regex("^(> )(.*)", RegexOption.MULTILINE).findAll(text).forEach { match ->
                                                val syntax = match.groups[1]!!
                                                builder.addStyle(SpanStyle(color = Color.Gray), match.range.first, match.range.last + 1)
                                                applySyntax(syntax.range)
                                            }

                                            Regex("^((\\d+\\.)|-)(\\s)(?!\\[[ xX]\\])", RegexOption.MULTILINE).findAll(text).forEach { match ->
                                                builder.addStyle(SpanStyle(color = primaryColor, fontWeight = FontWeight.Bold), match.range.first, match.range.last + 1)
                                            }

                                            Regex("\\*\\*(.*?)\\*\\*").findAll(text).forEach { match ->
                                                val content = match.groups[1]!!
                                                builder.addStyle(SpanStyle(fontWeight = FontWeight.Bold), content.range.first, content.range.last + 1)
                                                applySyntax(IntRange(match.range.first, content.range.first - 1))
                                                applySyntax(IntRange(content.range.last + 1, match.range.last))
                                            }

                                            Regex("(?<!\\*)\\*(?!\\*)(.*?)(?<!\\*)\\*(?!\\*)").findAll(text).forEach { match ->
                                                val content = match.groups[1]!!
                                                builder.addStyle(SpanStyle(fontStyle = FontStyle.Italic), content.range.first, content.range.last + 1)
                                                applySyntax(IntRange(match.range.first, content.range.first - 1))
                                                applySyntax(IntRange(content.range.last + 1, match.range.last))
                                            }

                                            Regex("~~(.*?)~~").findAll(text).forEach { match ->
                                                val content = match.groups[1]!!
                                                builder.addStyle(SpanStyle(textDecoration = TextDecoration.LineThrough), content.range.first, content.range.last + 1)
                                                applySyntax(IntRange(match.range.first, content.range.first - 1))
                                                applySyntax(IntRange(content.range.last + 1, match.range.last))
                                            }

                                            Regex("==(.*?)==").findAll(text).forEach { match ->
                                                val content = match.groups[1]!!
                                                builder.addStyle(SpanStyle(background = highlightColor, color = Color.Black), content.range.first, content.range.last + 1)
                                                applySyntax(IntRange(match.range.first, content.range.first - 1))
                                                applySyntax(IntRange(content.range.last + 1, match.range.last))
                                            }

                                            Regex("<u>(.*?)</u>").findAll(text).forEach { match ->
                                                val content = match.groups[1]!!
                                                builder.addStyle(SpanStyle(textDecoration = TextDecoration.Underline), content.range.first, content.range.last + 1)
                                                applySyntax(IntRange(match.range.first, content.range.first - 1))
                                                applySyntax(IntRange(content.range.last + 1, match.range.last))
                                            }

                                            TransformedText(builder.toAnnotatedString(), OffsetMapping.Identity)
                                        }
                                    }
                                )

                                val densityLocal = LocalDensity.current
                                checkboxRects.forEach { info ->
                                    Checkbox(
                                        checked = info.isChecked,
                                        onCheckedChange = { checked ->
                                            val text = textFieldValue.text
                                            val replacement = if (checked) "x" else " "
                                            val matchStr = text.substring(info.range)
                                            val newMatchStr = matchStr.replaceRange(3, 4, replacement)
                                            val newText = text.substring(0, info.range.first) + newMatchStr + text.substring(info.range.last + 1)
                                            val newSelection = if (textFieldValue.selection.start > info.range.last) textFieldValue.selection else textFieldValue.selection
                                            textFieldValue = textFieldValue.copy(text = newText, selection = newSelection)
                                            onContentChange(newText)
                                        },
                                        modifier = Modifier
                                            .absoluteOffset(
                                                x = with(densityLocal) { info.rect.left.toDp() } - 12.dp,
                                                y = with(densityLocal) { info.rect.center.y.toDp() } - 24.dp
                                            )
                                    )
                                }
                            }

                            Spacer(modifier = Modifier.height(50.dp))
                        }
                    }
                }
            }

            if (isTablet) {
                Row(modifier = Modifier.fillMaxSize()) {
                    Box(modifier = Modifier.width(300.dp).fillMaxHeight().verticalScroll(rememberScrollState())) {
                        TagSectionContent()
                    }
                    VerticalDivider()
                    Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
                        EditorSectionContent()
                    }
                }
            } else {
                Column(modifier = Modifier.fillMaxSize()) {
                    TagSectionContent()
                    EditorSectionContent()
                }
            }

            // 小屏幕：颜色选择弹窗
            if (showColorDialog) {
                AlertDialog(
                    onDismissRequest = { showColorDialog = false },
                    title = { Text("笔记颜色") },
                    text = {
                        NoteColorPickerContent(
                            currentHue = uiState.noteColorHue,
                            localHue = localColorHue,
                            onLocalHueChange = { localColorHue = it },
                            onColorChange = { onNoteColorHueChange(it) },
                            onClear = {
                                onNoteColorHueChange(null)
                                showColorDialog = false
                            },
                            customColors = customColors,
                            onAddPreset = { showAddPresetDialog = true },
                            onDeleteCustomColor = { index ->
                                deleteCustomColor(context, index)
                                customColors = loadCustomColors(context)
                            }
                        )
                    },
                    confirmButton = {},
                    dismissButton = {
                        TextButton(onClick = { showColorDialog = false }) {
                            Text("关闭")
                        }
                    }
                )
            }

            // 增加预设颜色弹窗
            if (showAddPresetDialog) {
                AddPresetColorDialog(
                    initialHue = localColorHue,
                    onDismiss = { showAddPresetDialog = false },
                    onSave = { hue, name ->
                        saveCustomColor(context, CustomColor(hue, name))
                        customColors = loadCustomColors(context)
                        showAddPresetDialog = false
                    }
                )
            }

            // 未保存提示弹窗
            if (showBackDialog) {
                AlertDialog(
                    onDismissRequest = { showBackDialog = false },
                    title = { Text("保存笔记") },
                    text = {
                        Column {
                            Text("您还未保存当前的笔记，请选择一个处理方式。")
                            Spacer(modifier = Modifier.height(12.dp))
                            // 笔记卡片预览
                            Surface(
                                shape = RoundedCornerShape(8.dp),
                                color = MaterialTheme.colorScheme.background,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column(modifier = Modifier.padding(12.dp)) {
                                    Text(
                                        text = uiState.content.take(200),
                                        style = MaterialTheme.typography.bodyMedium,
                                        maxLines = 4,
                                        overflow = TextOverflow.Ellipsis,
                                        color = MaterialTheme.colorScheme.onSurface,
                                    )
                                    val displayTags = NoteColorUtil.filterDisplayTags(uiState.tags)
                                    if (displayTags.isNotEmpty()) {
                                        Spacer(modifier = Modifier.height(4.dp))
                                        Text(
                                            text = displayTags.joinToString(" · "),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.primary,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                    }
                                }
                            }
                        }
                    },
                    confirmButton = {},
                    dismissButton = {
                        Column(
                            modifier = Modifier.fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Button(
                                onClick = {
                                    showBackDialog = false
                                    onSave()
                                },
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Text("保存")
                            }
                            Button(
                                onClick = {
                                    showBackDialog = false
                                },
                                modifier = Modifier.fillMaxWidth(),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.secondaryContainer,
                                    contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                                )
                            ) {
                                Text("继续编辑")
                            }
                            TextButton(
                                onClick = {
                                    showBackDialog = false
                                    onSaveDraft()
                                    hideKeyboardAndNavigate { onNavigateToHome() }
                                },
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Text("存到草稿箱")
                            }
                            TextButton(
                                onClick = {
                                    showBackDialog = false
                                    onDiscardDraft()
                                    hideKeyboardAndNavigate { onNavigateToHome() }
                                },
                                modifier = Modifier.fillMaxWidth(),
                                colors = ButtonDefaults.textButtonColors(
                                    contentColor = MaterialTheme.colorScheme.error
                                )
                            ) {
                                Text("废弃")
                            }
                        }
                    }
                )
            }

            // 进程意外终止后的恢复弹窗
            recoveryDraft?.let { draft ->
                AlertDialog(
                    onDismissRequest = {}, // 屏蔽点击空白关闭
                    title = { Text("有未保存的笔记") },
                    text = {
                        Column {
                            Text("检测到您上次编辑的笔记还未保存，我们已经为您自动保存到草稿箱。请选择一个处理上次笔记的方式：")
                            Spacer(modifier = Modifier.height(12.dp))
                            // 笔记卡片预览
                            Surface(
                                shape = RoundedCornerShape(8.dp),
                                color = MaterialTheme.colorScheme.background,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column(modifier = Modifier.padding(12.dp)) {
                                    Text(
                                        text = draft.content.take(200),
                                        style = MaterialTheme.typography.bodyMedium,
                                        maxLines = 4,
                                        overflow = TextOverflow.Ellipsis,
                                        color = MaterialTheme.colorScheme.onSurface,
                                    )
                                    val displayTags = NoteColorUtil.filterDisplayTags(draft.tags)
                                    if (displayTags.isNotEmpty()) {
                                        Spacer(modifier = Modifier.height(4.dp))
                                        Text(
                                            text = displayTags.joinToString(" · "),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.primary,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                    }
                                }
                            }
                        }
                    },
                    confirmButton = {},
                    dismissButton = {
                        Column(
                            modifier = Modifier.fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Button(
                                onClick = {
                                    // 继续编辑 - 加载草稿内容到编辑器
                                    onContentChange(draft.content)
                                    draft.tags.forEach { tag -> onAddTag(tag) }
                                    draft.noteColorHue?.let { onNoteColorHueChange(it) }
                                    draftStore.delete(draft.id)
                                    recoveryDraft = null
                                },
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Text("继续编辑")
                            }
                            Button(
                                onClick = {
                                    // 保存 - 加载草稿内容到编辑器，删除草稿，然后保存
                                    onContentChange(draft.content)
                                    draft.tags.forEach { tag -> onAddTag(tag) }
                                    draft.noteColorHue?.let { onNoteColorHueChange(it) }
                                    draftStore.delete(draft.id)
                                    recoveryDraft = null
                                    onSave()
                                },
                                modifier = Modifier.fillMaxWidth(),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.secondaryContainer,
                                    contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                                )
                            ) {
                                Text("保存")
                            }
                            Button(
                                onClick = {
                                    draftStore.delete(draft.id)
                                    recoveryDraft = null
                                    onRefreshDraftCount()
                                },
                                modifier = Modifier.fillMaxWidth(),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.error,
                                    contentColor = MaterialTheme.colorScheme.onError
                                )
                            ) {
                                Text("废弃")
                            }
                            TextButton(
                                onClick = {
                                    // 标记为已读
                                    onMarkDraftAsRead(draft.id)
                                    recoveryDraft = null
                                },
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Text("仍存储在草稿箱内（不推荐）")
                            }
                        }
                    }
                )
            }
        }
    }
}

@Composable
private fun NoteColorPickerContent(
    currentHue: Float?,
    localHue: Float,
    onLocalHueChange: (Float) -> Unit,
    onColorChange: (Float) -> Unit,
    onClear: () -> Unit,
    customColors: List<CustomColor>,
    onAddPreset: () -> Unit,
    onDeleteCustomColor: (Int) -> Unit,
) {
    Column {
        val presetHues = listOf(
            0f to "红",
            30f to "橙",
            55f to "黄",
            130f to "绿",
            210f to "蓝",
            270f to "紫",
        )

        FlowRow(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            for ((hue, label) in presetHues) {
                val color = NoteColorUtil.hueToColor(hue)
                val isSelected = currentHue != null && (currentHue - hue).let { it in -2f..2f }
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Box(
                        modifier = Modifier
                            .size(32.dp)
                            .background(color, CircleShape)
                            .clickable {
                                onLocalHueChange(hue)
                                onColorChange(hue)
                            }
                            .then(
                                if (isSelected) Modifier.border(2.dp, MaterialTheme.colorScheme.onSurface, CircleShape)
                                else Modifier
                            )
                    )
                    Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }

        if (customColors.isNotEmpty()) {
            Spacer(modifier = Modifier.height(8.dp))
            FlowRow(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for ((index, customColor) in customColors.withIndex()) {
                    val color = NoteColorUtil.hueToColor(customColor.hue)
                    val isSelected = currentHue != null && (currentHue - customColor.hue).let { it in -2f..2f }
                    InputChip(
                        selected = isSelected,
                        onClick = {
                            onLocalHueChange(customColor.hue)
                            onColorChange(customColor.hue)
                        },
                        label = { Text(customColor.name) },
                        leadingIcon = { Box(modifier = Modifier.size(12.dp).background(color, CircleShape)) },
                        trailingIcon = { Icon(Icons.Filled.Close, null, Modifier.size(InputChipDefaults.AvatarSize).clickable { onDeleteCustomColor(index) }) }
                    )
                }
                InputChip(selected = false, onClick = onAddPreset, label = { Text("增加预设颜色") }, trailingIcon = { Icon(Icons.Filled.Add, null, Modifier.size(16.dp)) })
            }
        } else {
            Spacer(modifier = Modifier.height(8.dp))
            InputChip(selected = false, onClick = onAddPreset, label = { Text("增加预设颜色") }, trailingIcon = { Icon(Icons.Filled.Add, null, Modifier.size(16.dp)) })
        }

        if (currentHue != null) {
            Spacer(modifier = Modifier.height(8.dp))
            Button(
                onClick = onClear,
                modifier = Modifier.fillMaxWidth(),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                )
            ) {
                Text("清除颜色")
            }
        }
    }
}

@Composable
private fun AddPresetColorDialog(
    initialHue: Float,
    onDismiss: () -> Unit,
    onSave: (Float, String) -> Unit,
) {
    var hue by remember { mutableFloatStateOf(initialHue) }
    var name by remember { mutableStateOf("") }
    val context = LocalContext.current

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("增加预设颜色") },
        text = {
            Column {
                val previewColor = NoteColorUtil.hueToColor(hue)
                Slider(
                    value = hue,
                    onValueChange = { hue = it },
                    valueRange = 0f..360f,
                    colors = SliderDefaults.colors(
                        thumbColor = previewColor,
                        activeTrackColor = previewColor,
                    )
                )

                Spacer(modifier = Modifier.height(16.dp))

                OutlinedTextField(
                    value = name,
                    onValueChange = { if (it.length <= 10) name = it },
                    label = { Text("颜色名称（选填）") },
                    placeholder = { Text(hue.toInt().toString()) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val finalName = name.ifBlank { hue.toInt().toString() }
                    if (finalName.length > 10) {
                        Toast.makeText(context, "颜色名称超出10个字符", Toast.LENGTH_SHORT).show()
                        return@TextButton
                    }
                    onSave(hue, finalName)
                },
                colors = ButtonDefaults.textButtonColors(
                    contentColor = MaterialTheme.colorScheme.primary
                )
            ) {
                Text("保存")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("取消")
            }
        }
    )
}