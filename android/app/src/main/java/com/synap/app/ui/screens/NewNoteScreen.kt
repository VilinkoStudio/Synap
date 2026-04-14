package com.synap.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.isImeVisible
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.material.icons.filled.Title
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
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
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
import kotlinx.coroutines.delay

enum class EditorSubMenu { NONE, HEADING, LIST }

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
    var tagInputHasFocus by remember { mutableStateOf(false) }
    val tagFocusRequester = remember { FocusRequester() }

    val isImeVisible = WindowInsets.isImeVisible
    var activeSubMenu by remember { mutableStateOf(EditorSubMenu.NONE) }

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
        activeSubMenu = EditorSubMenu.NONE
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
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

                // 标签区域 (保持原样)
                Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)) {
                    if (isTagInputVisible) {
                        OutlinedTextField(
                            value = tagInputText,
                            onValueChange = { tagInputText = it },
                            placeholder = { Text("输入标签") },
                            modifier = Modifier.fillMaxWidth().height(56.dp).focusRequester(tagFocusRequester)
                                .onFocusChanged { if (!it.isFocused && tagInputText.isBlank()) isTagInputVisible = false },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = {
                                if (tagInputText.isNotBlank()) { onAddTag(tagInputText.trim()); tagInputText = ""; isTagInputVisible = false }
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

            // 底部工具栏
            Column(
                modifier = Modifier.align(Alignment.BottomCenter).fillMaxWidth().imePadding().padding(bottom = if (isImeVisible) 8.dp else 24.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                // 二级菜单 (H1-H4, List 类型)
                AnimatedVisibility(visible = activeSubMenu != EditorSubMenu.NONE) {
                    Surface(modifier = Modifier.padding(bottom = 12.dp), shape = RoundedCornerShape(50), color = MaterialTheme.colorScheme.surfaceContainerHighest, shadowElevation = 6.dp) {
                        Row(modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp), Arrangement.spacedBy(4.dp)) {
                            when (activeSubMenu) {
                                EditorSubMenu.HEADING -> {
                                    (1..4).forEach { level -> TextButton(onClick = { applyLinePrefix("#".repeat(level) + " ") }) { Text("H$level") } }
                                }
                                EditorSubMenu.LIST -> {
                                    TextButton(onClick = { applyLinePrefix("- ") }) { Text("无序") }
                                    TextButton(onClick = { applyLinePrefix("1. ") }) { Text("有序") }
                                }
                                else -> {}
                            }
                        }
                    }
                }

                // 主工具栏
                Surface(shape = RoundedCornerShape(50), color = MaterialTheme.colorScheme.surfaceContainerHigh, shadowElevation = 6.dp, modifier = Modifier.padding(horizontal = 16.dp)) {
                    Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 6.dp).horizontalScroll(rememberScrollState()), verticalAlignment = Alignment.CenterVertically) {
                        val iconColor = MaterialTheme.colorScheme.onSurfaceVariant
                        IconButton(onClick = { applyStyle("**", "**") }) { Icon(Icons.Filled.FormatBold, null, tint = iconColor) }
                        IconButton(onClick = { applyStyle("*", "*") }) { Icon(Icons.Filled.FormatItalic, null, tint = iconColor) }
                        IconButton(onClick = { applyStyle("~~", "~~") }) { Icon(Icons.Filled.FormatStrikethrough, null, tint = iconColor) }
                        IconButton(onClick = { applyStyle("<u>", "</u>") }) { Icon(Icons.Filled.FormatUnderlined, null, tint = iconColor) }
                        IconButton(onClick = { applyStyle("==", "==") }) { Icon(Icons.Filled.FormatColorText, null, tint = iconColor) }
                        IconButton(onClick = { activeSubMenu = if (activeSubMenu == EditorSubMenu.HEADING) EditorSubMenu.NONE else EditorSubMenu.HEADING }) { Icon(Icons.Filled.Title, null, tint = iconColor) }
                        IconButton(onClick = { applyLinePrefix("> ") }) { Icon(Icons.Filled.FormatQuote, null, tint = iconColor) }
                        IconButton(onClick = { activeSubMenu = if (activeSubMenu == EditorSubMenu.LIST) EditorSubMenu.NONE else EditorSubMenu.LIST }) { Icon(Icons.Filled.FormatListBulleted, null, tint = iconColor) }
                    }
                }
            }
        }
    }
}