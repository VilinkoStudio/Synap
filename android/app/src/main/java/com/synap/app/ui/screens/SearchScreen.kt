package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridItemSpan
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.NoteCardItem
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.SearchResultNote
import com.synap.app.ui.model.SearchSourceBadge
import com.synap.app.ui.viewmodel.HomeUiState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay

@OptIn(ExperimentalSharedTransitionApi::class)
@Composable
fun SearchScreen(
    uiState: HomeUiState,
    onSearchQueryChange: (String) -> Unit,
    onSubmitSearch: () -> Unit,
    onClearSearch: () -> Unit,
    onNavigateBack: () -> Unit,
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    sharedTransitionScope: SharedTransitionScope? = null,
    animatedVisibilityScope: AnimatedVisibilityScope? = null,
) {
    val focusRequester = remember { FocusRequester() }
    val gridState = rememberLazyStaggeredGridState()
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("synap_prefs", android.content.Context.MODE_PRIVATE) }
    val dualColumnCards = remember { prefs.getBoolean("dual_column_cards", true) }

    // 进场动画后自动弹出键盘
    LaunchedEffect(Unit) {
        delay(300)
        focusRequester.requestFocus()
    }

    // ========== 预返回手势核心状态 ==========
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress
            }
            onNavigateBack()
        } catch (e: CancellationException) {
            backProgress = 0f
        }
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            // ========== 侧滑时的预返回手势缩放保留在最外层 ==========
            .graphicsLayer {
                translationX = backProgress * 64.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            },
        topBar = {
            Surface(
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 2.dp,
                modifier = Modifier
                    .fillMaxWidth()
                    // ========== 把展开动画的焦点从全屏移到顶部的搜索栏上 ==========
                    .let {
                        if (sharedTransitionScope != null && animatedVisibilityScope != null) {
                            with(sharedTransitionScope) {
                                it.sharedBounds(
                                    sharedContentState = rememberSharedContentState(key = "search_to_fullscreen"),
                                    animatedVisibilityScope = animatedVisibilityScope
                                )
                            }
                        } else {
                            it
                        }
                    }
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .statusBarsPadding()
                        .height(64.dp)
                        .padding(horizontal = 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    IconButton(onClick = {
                        onClearSearch()
                        onNavigateBack()
                    }) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }

                    TextField(
                        value = uiState.query,
                        onValueChange = {
                            onSearchQueryChange(it)
                            // 实时触发搜索
                            if (it.isNotBlank()) onSubmitSearch()
                        },
                        modifier = Modifier
                            .weight(1f)
                            .focusRequester(focusRequester),
                        placeholder = { Text(stringResource(R.string.search_placeholder)) },
                        singleLine = true,
                        colors = TextFieldDefaults.colors(
                            focusedContainerColor = Color.Transparent,
                            unfocusedContainerColor = Color.Transparent,
                            focusedIndicatorColor = Color.Transparent,
                            unfocusedIndicatorColor = Color.Transparent
                        ),
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                        keyboardActions = KeyboardActions(onSearch = { onSubmitSearch() })
                    )

                    if (uiState.query.isNotEmpty()) {
                        IconButton(onClick = onClearSearch) {
                            Icon(Icons.Filled.Clear, contentDescription = stringResource(R.string.clear))
                        }
                    }
                }
            }
        }
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(top = innerPadding.calculateTopPadding())
        ) {
            when {
                // 默认状态：输入框为空时不展示任何笔记
                uiState.query.isEmpty() -> {
                    Text(
                        text = stringResource(R.string.search_empty_input_hint),
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 加载中
                uiState.isLoading -> {
                    CircularProgressIndicator(
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 报错
                uiState.errorMessage != null -> {
                    Text(
                        text = uiState.errorMessage,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 无结果
                uiState.searchResults.isEmpty() -> {
                    Text(
                        text = buildAnnotatedString {
                            append("未搜索到和")
                            withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)) {
                                append("\"${uiState.query}\"")
                            }
                            append("相关的笔记")
                        },
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.align(Alignment.Center)
                    )
                }

                // 展示搜索结果
                else -> {
                    // 分离精准搜索和模糊搜索结果
                    val exactResults = uiState.searchResults.filter { result ->
                        result.sources.any { it == SearchSourceBadge.Semantic }
                    }
                    val fuzzyResults = uiState.searchResults.filter { result ->
                        result.sources.all { it == SearchSourceBadge.Fuzzy }
                    }

                    LazyVerticalStaggeredGrid(
                        columns = if (dualColumnCards) StaggeredGridCells.Fixed(2) else StaggeredGridCells.Adaptive(minSize = 240.dp),
                        state = gridState,
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(
                            start = 16.dp,
                            top = 16.dp,
                            end = 16.dp,
                            bottom = 16.dp + innerPadding.calculateBottomPadding()
                        ),
                        verticalItemSpacing = 16.dp,
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        // 精准搜索标题和结果
                        if (exactResults.isNotEmpty()) {
                            item(
                                key = "exact_search_header",
                                span = StaggeredGridItemSpan.FullLine,
                            ) {
                                Text(
                                    text = buildAnnotatedString {
                                        append("以下是包含关键词")
                                        withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)) {
                                            append("\"${uiState.query}\"")
                                        }
                                        append("的搜索结果")
                                    },
                                    style = MaterialTheme.typography.titleMedium,
                                    color = MaterialTheme.colorScheme.onSurface,
                                    modifier = Modifier.padding(bottom = 8.dp)
                                )
                            }

                            itemsIndexed(
                                exactResults,
                                key = { _, result -> result.note.id },
                            ) { index, result ->
                                SearchResultCard(
                                    result = result,
                                    onOpenNote = onOpenNote,
                                    onToggleDeleted = onToggleDeleted,
                                    animationDelayMillis = (index.coerceAtMost(6)) * 45,
                                    maxLines = if (dualColumnCards) 7 else 4,
                                )
                            }
                        }

                        // 模糊搜索标题和结果
                        if (fuzzyResults.isNotEmpty()) {
                            item(
                                key = "fuzzy_search_header",
                                span = StaggeredGridItemSpan.FullLine,
                            ) {
                                Spacer(modifier = Modifier.height(if (exactResults.isNotEmpty()) 16.dp else 0.dp))
                                Text(
                                    text = buildAnnotatedString {
                                        append("以下是")
                                        withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)) {
                                            append("\"${uiState.query}\"")
                                        }
                                        append("的相关搜索结果")
                                    },
                                    style = MaterialTheme.typography.titleMedium,
                                    color = MaterialTheme.colorScheme.onSurface,
                                    modifier = Modifier.padding(bottom = 8.dp)
                                )
                            }

                            itemsIndexed(
                                fuzzyResults,
                                key = { _, result -> result.note.id },
                            ) { index, result ->
                                SearchResultCard(
                                    result = result,
                                    onOpenNote = onOpenNote,
                                    onToggleDeleted = onToggleDeleted,
                                    animationDelayMillis = ((index + exactResults.size).coerceAtMost(6)) * 45,
                                    maxLines = if (dualColumnCards) 7 else 4,
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
private fun SearchResultCard(
    result: SearchResultNote,
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    animationDelayMillis: Int,
    maxLines: Int = 4,
) {
    NoteCardItem(
        note = result.note,
        onClick = { onOpenNote(result.note.id) },
        onLongClick = { },
        isSelectionMode = false,
        isSelected = false,
        onToggleDeleted = { onToggleDeleted(result.note) },
        onReply = { },
        animationDelayMillis = animationDelayMillis,
        maxLines = maxLines,
    )
}
