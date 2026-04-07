package com.synap.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.isImeVisible
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Reply
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
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.stringResource
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteTextSize
import com.synap.app.R
import com.synap.app.ui.viewmodel.EditorMode
import com.synap.app.ui.viewmodel.EditorUiState
import kotlinx.coroutines.delay

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun NewNoteScreen(
    uiState: EditorUiState,
    onNavigateBack: () -> Unit,
    onContentChange: (String) -> Unit,
    onAddTag: (String) -> Unit,
    onRemoveTag: (Int) -> Unit,
    onSave: () -> Unit,
) {
    var tagInputText by remember { mutableStateOf("") }
    var isTagInputVisible by remember { mutableStateOf(false) }
    var tagInputHasFocus by remember { mutableStateOf(false) }

    val bodyFocusRequester = remember { FocusRequester() }
    val tagFocusRequester = remember { FocusRequester() }
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
                            EditorMode.Create -> stringResource(R.string.edit_title_creat)
                            is EditorMode.Reply -> stringResource(R.string.edit_title_reply)
                            is EditorMode.Edit -> stringResource(R.string.edit_title_edit)
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
                .consumeWindowInsets(innerPadding)
                .padding(horizontal = 16.dp),
        ) {
            when (val mode = uiState.mode) {
                is EditorMode.Reply -> {
                    if (!mode.parentSummary.isNullOrBlank()) {
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f),
                            shape = MaterialTheme.shapes.medium,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(bottom = 12.dp),
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                                verticalAlignment = Alignment.Top,
                            ) {
                                Surface(
                                    shape = CircleShape,
                                    color = MaterialTheme.colorScheme.primaryContainer,
                                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                    modifier = Modifier.size(32.dp),
                                ) {
                                    Box(contentAlignment = Alignment.Center) {
                                        Icon(
                                            imageVector = Icons.Filled.Reply,
                                            contentDescription = null,
                                            modifier = Modifier.size(18.dp),
                                        )
                                    }
                                }

                                Column(
                                    modifier = Modifier.weight(1f),
                                    verticalArrangement = Arrangement.spacedBy(4.dp),
                                ) {
                                    Text(
                                        text = stringResource(R.string.edit_reply_context_title),
                                        style = MaterialTheme.typography.labelLarge,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                    Text(
                                        text = mode.parentSummary,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 2,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                            }
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
                    // --- 核心修改：应用字体和字重 ---
                    textStyle = MaterialTheme.typography.bodyLarge.copy(
                        fontFamily = LocalNoteFontFamily.current,
                        fontWeight = LocalNoteFontWeight.current,
                        fontSize = LocalNoteTextSize.current,
                        lineHeight = LocalNoteTextSize.current * 1.5f,
                        color = MaterialTheme.colorScheme.onSurface
                    ),
                    placeholder = { Text(stringResource(R.string.edit_placeholder)) },
                    colors = TextFieldDefaults.colors(
                        focusedContainerColor = Color.Transparent,
                        unfocusedContainerColor = Color.Transparent,
                        focusedIndicatorColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                    ),
                )
            }

            if (uiState.recommendedTags.isNotEmpty()) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = stringResource(R.string.edit_tag_recommend_title),
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )

                        if (uiState.isRecommendingTags) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(16.dp),
                                strokeWidth = 2.dp,
                            )
                        }
                    }

                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        uiState.recommendedTags.forEach { tag ->
                            SuggestionChip(
                                onClick = { onAddTag(tag) },
                                label = { Text(tag) },
                                icon = {
                                    Icon(
                                        imageVector = Icons.Filled.Add,
                                        contentDescription = null,
                                        modifier = Modifier.size(SuggestionChipDefaults.IconSize),
                                    )
                                },
                            )
                        }
                    }
                }
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .imePadding()
                    .padding(bottom = if (isImeVisible) 0.dp else 16.dp)
            ) {
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(bottom = 8.dp),
                ) {
                    uiState.tags.forEachIndexed { index, tag ->
                        InputChip(
                            selected = true,
                            onClick = { },
                            label = { Text(tag) },
                            trailingIcon = {
                                Icon(
                                    imageVector = Icons.Filled.Close,
                                    contentDescription = stringResource(R.string.edit_tag_delete),
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
                            label = { Text(stringResource(R.string.edit_tag_add)) },
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
                        placeholder = { Text(stringResource(R.string.edit_tag_placeholder)) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .focusRequester(tagFocusRequester)
                            .onFocusChanged { focusState ->
                                if (focusState.isFocused) {
                                    tagInputHasFocus = true
                                } else {
                                    if (tagInputHasFocus && tagInputText.isBlank()) {
                                        isTagInputVisible = false
                                    }
                                    tagInputHasFocus = false
                                }
                            },
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
                                    Icon(Icons.Filled.Check, contentDescription = stringResource(R.string.edit_tag_check))
                                }
                            }
                        }
                    )
                }
            }
        }
    }
}
