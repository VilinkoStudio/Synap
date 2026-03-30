package com.fuwaki.synap.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.fuwaki.synap.ui.components.NoteCardItem
import com.fuwaki.synap.ui.model.Note // 新增：导入 Note 模型
import com.fuwaki.synap.ui.viewmodel.HomeUiState
import kotlinx.coroutines.delay

@Composable
fun SearchScreen(
    uiState: HomeUiState,
    onSearchQueryChange: (String) -> Unit,
    onSubmitSearch: () -> Unit,
    onClearSearch: () -> Unit,
    onNavigateBack: () -> Unit,
    onOpenNote: (String) -> Unit,
    // --- 新增：接收删除回调 ---
    onToggleDeleted: (Note) -> Unit
) {
    // 1. 创建焦点请求器，用于控制键盘
    val focusRequester = remember { FocusRequester() }
    val gridState = rememberLazyStaggeredGridState()

    // 2. 页面进入时，延迟 300ms 等待滑入动画结束，然后自动弹出键盘
    LaunchedEffect(Unit) {
        delay(300)
        focusRequester.requestFocus()
    }

    Scaffold(
        topBar = {
            Surface(
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 2.dp,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .statusBarsPadding() // 避开状态栏
                        .height(64.dp)
                        .padding(horizontal = 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // 返回按钮
                    IconButton(onClick = {
                        onClearSearch() // 返回时清空搜索状态
                        onNavigateBack()
                    }) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
                    }

                    // 搜索输入框
                    TextField(
                        value = uiState.query,
                        onValueChange = onSearchQueryChange,
                        modifier = Modifier
                            .weight(1f)
                            .focusRequester(focusRequester), // 绑定焦点
                        placeholder = { Text("搜索笔记、标签、片段...") },
                        singleLine = true,
                        colors = TextFieldDefaults.colors(
                            focusedContainerColor = Color.Transparent,
                            unfocusedContainerColor = Color.Transparent,
                            focusedIndicatorColor = Color.Transparent,
                            unfocusedIndicatorColor = Color.Transparent
                        ),
                        // 把键盘的“回车”键变成“搜索”键
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                        keyboardActions = KeyboardActions(onSearch = { onSubmitSearch() })
                    )

                    // 清空按钮（只有输入了文字才显示）
                    if (uiState.query.isNotEmpty()) {
                        IconButton(onClick = onClearSearch) {
                            Icon(Icons.Filled.Clear, contentDescription = "清空")
                        }
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
            when {
                // 正在搜索时显示加载圈
                uiState.isLoading && uiState.query.isNotEmpty() -> {
                    CircularProgressIndicator(
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 搜索出错
                uiState.errorMessage != null -> {
                    Text(
                        text = uiState.errorMessage,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 搜索完成，但没找到结果
                uiState.notes.isEmpty() && uiState.query.isNotEmpty() && !uiState.isLoading -> {
                    Text(
                        text = "没有找到匹配的笔记",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 搜索出结果了，展示瀑布流
                uiState.notes.isNotEmpty() -> {
                    LazyVerticalStaggeredGrid(
                        columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
                        state = gridState,
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(16.dp),
                        verticalItemSpacing = 16.dp,
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        itemsIndexed(uiState.notes, key = { _, note -> note.id }) { index, note ->
                            NoteCardItem(
                                note = note,
                                onClick = { onOpenNote(note.id) },
                                // --- 修改：将接收到的删除回调传给卡片 ---
                                onToggleDeleted = { onToggleDeleted(note) },
                                onReply = { }, // 为了保持搜索页整洁，暂时禁用了卡片上的直接回复
                                animationDelayMillis = (index.coerceAtMost(6)) * 45,
                            )
                        }
                    }
                }

                // 刚进页面，还没输入任何内容
                else -> {
                    Text(
                        text = "输入关键词开始搜索",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.align(Alignment.Center)
                    )
                }
            }
        }
    }
}