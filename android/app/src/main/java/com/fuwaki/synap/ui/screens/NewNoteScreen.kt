package com.fuwaki.synap.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.fuwaki.synap.ui.viewmodel.EditorMode
import com.fuwaki.synap.ui.viewmodel.EditorUiState

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
    var showTagDialog by remember { mutableStateOf(false) }
    var editingTagIndex by remember { mutableIntStateOf(-1) }
    var tagInputText by remember { mutableStateOf("") }

    if (showTagDialog) {
        AlertDialog(
            onDismissRequest = { showTagDialog = false },
            title = { Text(if (editingTagIndex == -1) "添加标签" else "编辑标签") },
            text = {
                OutlinedTextField(
                    value = tagInputText,
                    onValueChange = { tagInputText = it },
                    singleLine = true,
                    placeholder = { Text("输入标签名称") },
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        val normalized = tagInputText.trim()
                        if (editingTagIndex == -1) {
                            onAddTag(normalized)
                        } else if (normalized.isEmpty()) {
                            onRemoveTag(editingTagIndex)
                        } else {
                            onUpdateTag(editingTagIndex, normalized)
                        }
                        showTagDialog = false
                    },
                ) {
                    Text("确定")
                }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        if (editingTagIndex == -1) {
                            showTagDialog = false
                        } else {
                            onRemoveTag(editingTagIndex)
                            showTagDialog = false
                        }
                    },
                ) {
                    Text(if (editingTagIndex == -1) "取消" else "删除")
                }
            },
        )
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

            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                uiState.tags.forEachIndexed { index, tag ->
                    InputChip(
                        selected = false,
                        onClick = {
                            editingTagIndex = index
                            tagInputText = tag
                            showTagDialog = true
                        },
                        label = { Text(tag) },
                    )
                }
                InputChip(
                    selected = false,
                    onClick = {
                        editingTagIndex = -1
                        tagInputText = ""
                        showTagDialog = true
                    },
                    label = { Text("添加标签") },
                    leadingIcon = {
                        Icon(
                            Icons.Filled.Add,
                            contentDescription = null,
                        )
                    },
                )
            }

            Spacer(modifier = Modifier.height(8.dp))
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            if (uiState.isLoading) {
                CircularProgressIndicator(modifier = Modifier.padding(top = 24.dp))
            } else {
                TextField(
                    value = uiState.content,
                    onValueChange = onContentChange,
                    modifier = Modifier.fillMaxSize(),
                    placeholder = { Text("开始输入正文...") },
                    colors = TextFieldDefaults.colors(
                        focusedContainerColor = Color.Transparent,
                        unfocusedContainerColor = Color.Transparent,
                        focusedIndicatorColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                    ),
                )
            }
        }
    }
}
