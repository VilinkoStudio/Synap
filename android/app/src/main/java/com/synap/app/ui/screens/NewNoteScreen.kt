package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.isImeVisible
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.FormatBold
import androidx.compose.material.icons.filled.FormatColorText
import androidx.compose.material.icons.filled.FormatItalic
import androidx.compose.material.icons.filled.FormatListBulleted
import androidx.compose.material.icons.filled.FormatQuote
import androidx.compose.material.icons.filled.FormatStrikethrough
import androidx.compose.material.icons.filled.FormatUnderlined
import androidx.compose.material.icons.filled.KeyboardArrowLeft
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.InputChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.appcompat.widget.AppCompatEditText
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import com.synap.app.LocalNoteTextSize
import com.synap.app.R
import com.synap.app.ui.viewmodel.EditorMode
import com.synap.app.ui.viewmodel.EditorUiState
import io.noties.markwon.Markwon
import io.noties.markwon.editor.MarkwonEditor
import io.noties.markwon.editor.MarkwonEditorTextWatcher
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class, ExperimentalSharedTransitionApi::class)
@Composable
fun NewNoteScreen(
    uiState: EditorUiState,
    onNavigateBack: () -> Unit,
    onContentChange: (String) -> Unit,
    onAddTag: (String) -> Unit,
    onRemoveTag: (Int) -> Unit,
    onSave: () -> Unit,
    sharedTransitionScope: SharedTransitionScope? = null,
    animatedVisibilityScope: AnimatedVisibilityScope? = null,
) {
    var tagInputText by remember { mutableStateOf("") }
    var isTagInputVisible by remember { mutableStateOf(false) }
    val tagFocusRequester = remember { FocusRequester() }

    val isImeVisible = WindowInsets.isImeVisible

    // 控制工具栏展开和收起的状态
    var isToolbarExpanded by remember { mutableStateOf(true) }

    // 引用原生的 EditText 实例以便从工具栏操作
    var nativeEditText by remember { mutableStateOf<AppCompatEditText?>(null) }

    // 工具栏逻辑适配：操作原生 EditText
    val applyStyle: (String, String) -> Unit = { prefix, suffix ->
        nativeEditText?.let { et ->
            val start = et.selectionStart
            val end = et.selectionEnd
            val text = et.text ?: return@let
            if (start == end) {
                text.insert(start, prefix + suffix)
                et.setSelection(start + prefix.length)
            } else {
                text.insert(end, suffix)
                text.insert(start, prefix)
            }
        }
    }

    val applyLinePrefix: (String) -> Unit = { prefix ->
        nativeEditText?.let { et ->
            val start = et.selectionStart
            val text = et.text ?: return@let
            val layout = et.layout
            val line = layout.getLineForOffset(start)
            val lineStart = layout.getLineStart(line)
            text.insert(lineStart, prefix)
        }
    }

    // ========== 预返回手势核心状态 ==========
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress // 收集滑动进度
            }
            onNavigateBack() // 手指松开且决定返回时，触发导航
        } catch (e: CancellationException) {
            backProgress = 0f // 用户取消了返回手势，重置进度
        }
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            // ========== 应用预返回手势的视觉形变 ==========
            .graphicsLayer {
                val scale = 1f - (0.1f * backProgress) // 页面最多缩小到 90%
                scaleX = scale
                scaleY = scale
                shape = RoundedCornerShape(32.dp * backProgress) // 随进度增加圆角
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
                title = { Text(stringResource(if (uiState.mode is EditorMode.Create) R.string.edit_title_creat else R.string.edit_title_edit)) },
                navigationIcon = { IconButton(onClick = onNavigateBack) { Icon(Icons.Filled.ArrowBack, "返回") } },
                actions = {
                    IconButton(onClick = onSave, enabled = !uiState.isSaving && !uiState.isLoading) {
                        if (uiState.isSaving) CircularProgressIndicator(modifier = Modifier.size(24.dp)) else Icon(Icons.Filled.Check, "保存")
                    }
                },
            )
        },
    ) { innerPadding ->
        Box(modifier = Modifier.fillMaxSize()) {
            Column(modifier = Modifier.fillMaxSize().padding(innerPadding).consumeWindowInsets(innerPadding)) {

                // 标签区域
                Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)) {
                    if (isTagInputVisible) {
                        // 【核心修复】：出现输入框时，强制请求焦点，防止触发 onFocusChanged 导致瞬间隐藏
                        var hasGainedFocus by remember { mutableStateOf(false) }

                        LaunchedEffect(Unit) {
                            try {
                                tagFocusRequester.requestFocus()
                            } catch (e: Exception) {
                                // 忽略偶发的焦点请求异常
                            }
                        }

                        OutlinedTextField(
                            value = tagInputText,
                            onValueChange = { tagInputText = it },
                            placeholder = { Text("输入标签") },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(56.dp)
                                .focusRequester(tagFocusRequester)
                                .onFocusChanged { focusState ->
                                    if (focusState.isFocused) {
                                        // 成功获取到焦点，打上标记
                                        hasGainedFocus = true
                                    } else if (hasGainedFocus && tagInputText.isBlank()) {
                                        // 只有在“曾经获取过焦点”，并且现在失去焦点且内容为空时，才隐藏输入框
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
                    } else {
                        Row(modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            uiState.tags.forEachIndexed { i, tag ->
                                InputChip(selected = true, onClick = {}, label = { Text(tag) },
                                    trailingIcon = { Icon(Icons.Filled.Close, null, Modifier.size(InputChipDefaults.AvatarSize).clickable { onRemoveTag(i) }) })
                            }
                            InputChip(selected = false, onClick = { isTagInputVisible = true }, label = { Text("添加标签") }, trailingIcon = { Icon(Icons.Filled.Add, null, Modifier.size(16.dp)) })
                        }
                    }
                    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                }

                // 正文编辑区：Markwon + AndroidView
                Box(modifier = Modifier.weight(1f).padding(horizontal = 16.dp)) {
                    if (uiState.isLoading) {
                        CircularProgressIndicator(modifier = Modifier.align(Alignment.Center))
                    } else {
                        val textColor = MaterialTheme.colorScheme.onSurface.toArgb()
                        val hintColor = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f).toArgb()
                        val fontSize = LocalNoteTextSize.current.value

                        AndroidView(
                            modifier = Modifier.fillMaxSize(),
                            factory = { context ->
                                AppCompatEditText(context).apply {
                                    nativeEditText = this
                                    background = null
                                    gravity = Gravity.TOP
                                    setPadding(0, 0, 0, 0)
                                    textSize = fontSize
                                    setTextColor(textColor)
                                    setHintTextColor(hintColor)
                                    hint = "开始记录你的灵感..."

                                    // 配置 Markwon
                                    val markwon = Markwon.create(context)
                                    val editor = MarkwonEditor.create(markwon)
                                    addTextChangedListener(MarkwonEditorTextWatcher.withProcess(editor))

                                    // 同步数据回 ViewModel
                                    addTextChangedListener(object : TextWatcher {
                                        override fun beforeTextChanged(s: CharSequence?, st: Int, c: Int, a: Int) {}
                                        override fun onTextChanged(s: CharSequence?, st: Int, b: Int, c: Int) {
                                            onContentChange(s?.toString() ?: "")
                                        }
                                        override fun afterTextChanged(s: Editable?) {}
                                    })

                                    setText(uiState.content)
                                    setSelection(uiState.content.length)
                                    requestFocus()
                                }
                            },
                            update = { view ->
                                // 仅在内容差异较大时更新，防止光标跳动
                                if (view.text.toString() != uiState.content) {
                                    view.setText(uiState.content)
                                }
                            }
                        )
                    }
                }
            }

            // 底部工具栏与展开收起按钮
            Box(
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .fillMaxWidth()
                    .imePadding()
                    .padding(horizontal = 16.dp)
                    .padding(bottom = if (isImeVisible) 8.dp else 24.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // 主工具栏 (带展开/收缩动画)
                    AnimatedVisibility(
                        visible = isToolbarExpanded,
                        enter = fadeIn() + expandHorizontally(expandFrom = Alignment.End),
                        exit = fadeOut() + shrinkHorizontally(shrinkTowards = Alignment.End),
                        modifier = Modifier.weight(1f, fill = false)
                    ) {
                        Surface(
                            shape = RoundedCornerShape(50),
                            color = MaterialTheme.colorScheme.primaryContainer,
                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                            shadowElevation = 6.dp
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 8.dp, vertical = 6.dp)
                                    .horizontalScroll(rememberScrollState()),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                val iconColor = MaterialTheme.colorScheme.onPrimaryContainer
                                val textStyle = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold, color = iconColor)

                                // 将所有快捷操作整合至同一行
                                IconButton(onClick = { applyLinePrefix("# ") }) { Text("H1", style = textStyle) }
                                IconButton(onClick = { applyLinePrefix("## ") }) { Text("H2", style = textStyle) }
                                IconButton(onClick = { applyStyle("**", "**") }) { Icon(Icons.Filled.FormatBold, null, tint = iconColor) }
                                IconButton(onClick = { applyStyle("*", "*") }) { Icon(Icons.Filled.FormatItalic, null, tint = iconColor) }
                                IconButton(onClick = { applyStyle("~~", "~~") }) { Icon(Icons.Filled.FormatStrikethrough, null, tint = iconColor) }
                                IconButton(onClick = { applyStyle("<u>", "</u>") }) { Icon(Icons.Filled.FormatUnderlined, null, tint = iconColor) }
                                IconButton(onClick = { applyStyle("==", "==") }) { Icon(Icons.Filled.FormatColorText, null, tint = iconColor) }
                                IconButton(onClick = { applyLinePrefix("> ") }) { Icon(Icons.Filled.FormatQuote, null, tint = iconColor) }
                                IconButton(onClick = { applyLinePrefix("- ") }) { Icon(Icons.Filled.FormatListBulleted, null, tint = iconColor) }
                                IconButton(onClick = { applyLinePrefix("1. ") }) { Text("1.", style = textStyle) }
                            }
                        }
                    }

                    Spacer(modifier = Modifier.width(8.dp))

                    // 独立的开关按钮
                    Surface(
                        shape = CircleShape,
                        color = MaterialTheme.colorScheme.primaryContainer,
                        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                        shadowElevation = 6.dp
                    ) {
                        IconButton(onClick = { isToolbarExpanded = !isToolbarExpanded }) {
                            Icon(
                                imageVector = if (isToolbarExpanded) Icons.Filled.Close else Icons.Filled.KeyboardArrowLeft,
                                contentDescription = if (isToolbarExpanded) "收起" else "展开"
                            )
                        }
                    }
                }
            }
        }
    }
}