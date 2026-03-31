package com.fuwaki.synap.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll // --- 新增：横向滚动 ---
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets // --- 新增：窗口插入（用于检测键盘） ---
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.isImeVisible // --- 新增：检测键盘是否可见 ---
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState // --- 新增：滚动状态 ---
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
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
import androidx.compose.material3.SuggestionChip
import androidx.compose.material3.SuggestionChipDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.fuwaki.synap.ui.viewmodel.EditorMode
import com.fuwaki.synap.ui.viewmodel.EditorUiState
import kotlinx.coroutines.delay

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun NewNoteScreen(
    uiState: EditorUiState,
    onNavigateBack: () -> Unit,
    onContentChange: (String) -> Unit,
    onAddTag: (String) -> Unit,
    onUpdateTag: (Int, String) -> Unit,
    onRemoveTag: (Int) -> Unit,
    onSave: () -> Unit,
) {
    var tagInputText by remember { mutableStateOf("") }
    var isTagInputVisible by remember { mutableStateOf(false) }
    val bodyFocusRequester = remember { FocusRequester() }
    val tagFocusRequester = remember { FocusRequester() }
    val tagScrollState = rememberScrollState()
    val isImeVisible = WindowInsets.isImeVisible

    LaunchedEffect(Unit) {
        delay(300)
        bodyFocusRequester.requestFocus()
    }

    LaunchedEffect(isTagInputVisible) {
        if (isTagInputVisible) {
            delay(100)
            tagFocusRequester.requestFocus()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        when (uiState.mode) {
                            EditorMode.Create -> "新建笔记"
                            is EditorMode.Reply -> "回复笔记"
                            is EditorMode.Edit -> "编辑笔记"
                        },
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
                actions = {
                    IconButton(
                        onClick = onSave,
                        enabled = !uiState.isSaving && !uiState.isLoading,
                    ) {
                        if (uiState.isSaving) {
                            CircularProgressIndicator(modifier = Modifier.padding(8.dp))
                        } else {
                            Icon(Icons.Filled.Check, contentDescription = "保存")
                        }
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 16.dp),
        ) {
            when (val mode = uiState.mode) {
                is EditorMode.Reply -> {
                    if (!mode.parentSummary.isNullOrBlank()) {
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            shape = MaterialTheme.shapes.small,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(bottom = 12.dp),
                        ) {
                            Text(
                                text = "回复自“${mode.parentSummary}”",
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                                maxLines = 2,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                }
                else -> Unit
            }

            if (uiState.errorMessage != null) {
                Text(
                    text = uiState.errorMessage,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(bottom = 12.dp),
                )
            }

            if (uiState.isLoading) {
                Box(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            } else {
                TextField(
                    value = uiState.content,
                    onValueChange = onContentChange,
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .focusRequester(bodyFocusRequester),
                    placeholder = { Text("开始输入正文...") },
                    colors = TextFieldDefaults.colors(
                        focusedContainerColor = Color.Transparent,
                        unfocusedContainerColor = Color.Transparent,
                        focusedIndicatorColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                    ),
                )
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .imePadding()
                    .padding(bottom = if (isImeVisible) 0.dp else 16.dp)
            ) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(tagScrollState)
                        .padding(bottom = 8.dp),
                ) {
                    uiState.tags.forEachIndexed { index, tag ->
                        InputChip(
                            selected = false,
                            onClick = { },
                            label = { Text(tag) },
                            trailingIcon = {
                                Icon(
                                    imageVector = Icons.Filled.Close,
                                    contentDescription = "删除标签",
                                    modifier = Modifier
                                        .size(InputChipDefaults.AvatarSize)
                                        .clickable { onRemoveTag(index) }
                                )
                            }
                        )
                    }

                    if (!isTagInputVisible) {
                        SuggestionChip(
                            onClick = { isTagInputVisible = true },
                            label = { Text(" 添加标签") },
                            icon = {
                                Icon(
                                    imageVector = Icons.Filled.Add,
                                    contentDescription = null,
                                    modifier = Modifier.size(SuggestionChipDefaults.IconSize)
                                )
                            }
                        )
                    }
                }

                if (isTagInputVisible) {
                    OutlinedTextField(
                        value = tagInputText,
                        onValueChange = { tagInputText = it },
                        placeholder = { Text("在此输入标签文字") },
                        modifier = Modifier
                            .fillMaxWidth()
                            .focusRequester(tagFocusRequester),
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(onDone = {
                            if (tagInputText.isNotBlank()) {
                                onAddTag(tagInputText.trim())
                                tagInputText = ""
                                isTagInputVisible = false
                            }
                        }),
                        trailingIcon = {
                            if (tagInputText.isNotBlank()) {
                                IconButton(onClick = {
                                    onAddTag(tagInputText.trim())
                                    tagInputText = ""
                                    isTagInputVisible = false
                                }) {
                                    Icon(Icons.Filled.Check, contentDescription = "确认添加")
                                }
                            }
                        }
                    )
                }
            }
        }
    }
}