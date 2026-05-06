package com.synap.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountTree
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Description
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FloatingToolbarDefaults
import androidx.compose.material3.HorizontalFloatingToolbar
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
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.synap.app.LocalNoteFontFamily
import com.synap.app.LocalNoteFontWeight
import com.synap.app.LocalNoteLineSpacing
import com.synap.app.LocalNoteTextSize
import com.synap.app.ui.components.DagCanvasOrientation
import com.synap.app.ui.components.ThreadDagCanvas
import com.synap.app.ui.model.Note
import com.synap.app.ui.util.NoteColorUtil
import com.synap.app.ui.model.ThreadBranchChoice
import com.synap.app.ui.viewmodel.ThreadReaderUiState

@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ThreadReaderScreen(
    uiState: ThreadReaderUiState,
    onNavigateBack: () -> Unit,
    onOpenOriginDetail: (String) -> Unit,
    onOpenBranch: (ThreadBranchChoice) -> Unit,
    onOpenNodeAsAnchor: (String) -> Unit,
    onShowBranchSheet: (List<ThreadBranchChoice>) -> Unit,
    onDismissBranchSheet: () -> Unit,
    onBacktrack: () -> Unit,
    onRefresh: () -> Unit,
    onOpenGraph: () -> Unit,
    onDismissGraph: () -> Unit,
    onFocusNode: (String) -> Unit,
) {
    val branchSheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val graphSheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    if (uiState.isBranchSheetVisible) {
        ModalBottomSheet(
            onDismissRequest = onDismissBranchSheet,
            sheetState = branchSheetState,
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("选择接着阅读的位置", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                uiState.activeBranchChoices.forEach { choice ->
                    BranchChoiceSheetCard(choice = choice, onClick = { onOpenBranch(choice) })
                }
                Spacer(modifier = Modifier.height(12.dp))
            }
        }
    }

    if (uiState.isGraphSheetVisible && uiState.segment != null) {
        ModalBottomSheet(
            onDismissRequest = onDismissGraph,
            sheetState = graphSheetState,
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("关系概览", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                ThreadDagCanvas(
                    graph = uiState.segment.graph,
                    onNodeClick = { node ->
                        if (node.isInPrimarySegment) {
                            onFocusNode(node.id)
                        } else {
                            onOpenNodeAsAnchor(node.id)
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(320.dp),
                )
            }
        }
    }

    BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
        val isWide = maxWidth >= 840.dp

        Scaffold(
            topBar = {
                TopAppBar(
                    title = {
                        Text(
                            text = "关系浏览",
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    },
                    navigationIcon = {
                        IconButton(onClick = onNavigateBack) {
                            Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
                        }
                    },
                    actions = {
                        IconButton(onClick = { onOpenOriginDetail(uiState.originNoteId) }) {
                            Icon(Icons.Filled.Description, contentDescription = "返回详情")
                        }
                        if (!isWide) {
                            IconButton(onClick = onOpenGraph) {
                                Icon(Icons.Filled.AccountTree, contentDescription = "打开关系概览")
                            }
                        }
                    },
                )
            },
        ) { innerPadding ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(MaterialTheme.colorScheme.surface)
                    .padding(innerPadding),
            ) {
                when {
                    uiState.isLoading && uiState.segment == null -> {
                        Column(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(24.dp),
                            verticalArrangement = Arrangement.Center,
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            androidx.compose.material3.CircularProgressIndicator()
                        }
                    }

                    uiState.segment == null -> {
                        Column(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(24.dp),
                            verticalArrangement = Arrangement.Center,
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text(uiState.errorMessage ?: "加载失败", color = MaterialTheme.colorScheme.error)
                            OutlinedButton(onClick = onRefresh, modifier = Modifier.padding(top = 16.dp)) {
                                Text("重试")
                            }
                        }
                    }

                    else -> {
                        val segment = checkNotNull(uiState.segment)
                        if (isWide) {
                            Row(
                                modifier = Modifier
                                    .fillMaxSize()
                                    .padding(horizontal = 12.dp, vertical = 8.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Surface(
                                    modifier = Modifier
                                        .weight(0.42f)
                                        .fillMaxSize(),
                                    shape = MaterialTheme.shapes.large,
                                    color = MaterialTheme.colorScheme.surfaceContainerLow,
                                ) {
                                    ThreadDagCanvas(
                                        graph = segment.graph,
                                        orientation = DagCanvasOrientation.Vertical,
                                        onNodeClick = { node ->
                                            if (node.isInPrimarySegment) {
                                                onFocusNode(node.id)
                                            } else {
                                                onOpenNodeAsAnchor(node.id)
                                            }
                                        },
                                        modifier = Modifier.fillMaxSize().padding(12.dp),
                                    )
                                }
                                Surface(
                                    modifier = Modifier
                                        .weight(0.58f)
                                        .fillMaxSize(),
                                    shape = MaterialTheme.shapes.large,
                                    color = Color.Transparent,
                                ) {
                                    ThreadLinearPane(
                                        uiState = uiState,
                                        onFocusNode = onFocusNode,
                                        onOpenBranch = onOpenBranch,
                                        onShowBranchSheet = onShowBranchSheet,
                                        modifier = Modifier.fillMaxSize(),
                                    )
                                }
                            }
                        } else {
                            ThreadLinearPane(
                                uiState = uiState,
                                onFocusNode = onFocusNode,
                                onOpenBranch = onOpenBranch,
                                onShowBranchSheet = onShowBranchSheet,
                                modifier = Modifier.fillMaxSize(),
                            )
                        }
                    }
                }

                AnimatedVisibility(
                    visible = uiState.segment != null,
                    enter = fadeIn(),
                    exit = fadeOut(),
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .navigationBarsPadding()
                        .padding(bottom = 20.dp),
                ) {
                    HorizontalFloatingToolbar(
                        expanded = true,
                        colors = FloatingToolbarDefaults.vibrantFloatingToolbarColors(
                            toolbarContainerColor = MaterialTheme.colorScheme.secondaryContainer,
                            toolbarContentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                        ),
                    ) {
                        TextButton(onClick = onBacktrack, enabled = uiState.historyDepth > 0) {
                            Text("返回上一分支")
                        }
                        TextButton(onClick = { onOpenOriginDetail(uiState.originNoteId) }) {
                            Text("查看原笔记")
                        }
                        FilledTonalButton(onClick = onRefresh) {
                            Text("刷新当前线程")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ThreadLinearPane(
    uiState: ThreadReaderUiState,
    onFocusNode: (String) -> Unit,
    onOpenBranch: (ThreadBranchChoice) -> Unit,
    onShowBranchSheet: (List<ThreadBranchChoice>) -> Unit,
    modifier: Modifier = Modifier,
) {
    val segment = checkNotNull(uiState.segment)
    val listState = rememberLazyListState()
    val focusedIndex = remember(segment.steps, segment.focusedNodeId) {
        segment.steps.indexOfFirst { it.note.id == segment.focusedNodeId }.takeIf { it >= 0 }
    }

    LaunchedEffect(focusedIndex, segment.anchorId) {
        val index = focusedIndex ?: return@LaunchedEffect
        if (listState.firstVisibleItemIndex != index) {
            listState.animateScrollToItem(index)
        }
    }

    LaunchedEffect(listState, segment.steps) {
        androidx.compose.runtime.snapshotFlow { listState.firstVisibleItemIndex }
            .collect { index ->
                segment.steps.getOrNull(index)?.note?.id?.let(onFocusNode)
            }
    }

    LazyColumn(
        modifier = modifier,
        state = listState,
        verticalArrangement = Arrangement.spacedBy(14.dp),
        contentPadding = PaddingValues(
            start = 16.dp,
            end = 16.dp,
            top = 12.dp,
            bottom = 120.dp,
        ),
    ) {
        itemsIndexed(segment.steps, key = { _, step -> step.note.id }) { index, step ->
            val previousStepId = segment.steps.getOrNull(index - 1)?.note?.id
            val nextStepId = segment.steps.getOrNull(index + 1)?.note?.id
            val visibleParentChoices = step.parentChoices.filterNot { it.note.id == previousStepId }
            val visibleChildChoices = step.childChoices.filterNot { it.note.id == nextStepId }

            if (visibleParentChoices.isNotEmpty()) {
                CompactBranchChoices(
                    title = "也可以从这里读来",
                    choices = visibleParentChoices,
                    placement = BranchPlacement.Before,
                    onOpenBranch = onOpenBranch,
                    onShowAll = onShowBranchSheet,
                )
            }

            ThreadStepCard(
                note = step.note,
                isFirst = index == 0,
                isFocused = step.note.id == segment.focusedNodeId,
            )

            if (visibleChildChoices.isNotEmpty()) {
                CompactBranchChoices(
                    title = "接着读",
                    choices = visibleChildChoices,
                    placement = BranchPlacement.After,
                    onOpenBranch = onOpenBranch,
                    onShowAll = onShowBranchSheet,
                )
            }
        }
    }
}

@Composable
private fun ThreadStepCard(
    note: Note,
    isFirst: Boolean,
    isFocused: Boolean,
) {
    val primaryColor = MaterialTheme.colorScheme.primary
    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
    val baseFontSize = LocalNoteTextSize.current.value
    val annotatedContent = remember(note.content, primaryColor, highlightColor, baseFontSize) {
        buildMarkdownAnnotatedString(note.content, primaryColor, highlightColor, baseFontSize, isCompact = false)
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = if (isFirst) RoundedCornerShape(28.dp) else RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(
            containerColor = when {
                isFocused -> MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.55f)
                else -> MaterialTheme.colorScheme.surfaceContainerLow
            },
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = if (isFocused) 3.dp else 1.dp),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 18.dp, vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = NoteColorUtil.filterDisplayTags(note.tags).joinToString(" · ").ifBlank { "无标签" },
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
            }

            Text(
                text = annotatedContent,
                style = MaterialTheme.typography.bodyLarge.copy(
                    fontFamily = LocalNoteFontFamily.current,
                    fontWeight = LocalNoteFontWeight.current,
                    fontSize = LocalNoteTextSize.current,
                    lineHeight = LocalNoteTextSize.current * LocalNoteLineSpacing.current,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

@Composable
private fun CompactBranchChoices(
    title: String,
    choices: List<ThreadBranchChoice>,
    placement: BranchPlacement,
    onOpenBranch: (ThreadBranchChoice) -> Unit,
    onShowAll: (List<ThreadBranchChoice>) -> Unit,
) {
    val recommendedChoice = choices.firstOrNull { it.isRecommended }
    val alternateChoices = choices.filterNot { it.note.id == recommendedChoice?.note?.id }
    val previewAlternates = alternateChoices.take(2)
    val collapsedCount = (alternateChoices.size - previewAlternates.size).coerceAtLeast(0)
    val showAlternateGroup = alternateChoices.isNotEmpty()

    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(if (showAlternateGroup && recommendedChoice != null) 8.dp else 0.dp),
    ) {
        recommendedChoice?.let { choice ->
            BranchPreviewCard(
                choice = choice,
                placement = placement,
                onClick = { onOpenBranch(choice) },
                onOpenContext = onOpenBranch,
            )
        }

        if (showAlternateGroup) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(
                        start = if (placement == BranchPlacement.After) 18.dp else 0.dp,
                        end = if (placement == BranchPlacement.Before) 18.dp else 0.dp,
                        top = if (recommendedChoice == null) 2.dp else 0.dp,
                        bottom = 6.dp,
                    ),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                if (placement == BranchPlacement.After) {
                    BranchConnector(placement = placement, modifier = Modifier.padding(top = 26.dp))
                }

                Column(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.weight(1f),
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 4.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = title,
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.primary,
                        )
                        if (collapsedCount > 0) {
                            Surface(
                                modifier = Modifier
                                    .clip(RoundedCornerShape(999.dp))
                                    .clickable { onShowAll(alternateChoices) },
                                color = MaterialTheme.colorScheme.surfaceContainerHighest,
                                shape = RoundedCornerShape(999.dp),
                            ) {
                                Text(
                                    text = "+$collapsedCount",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.primary,
                                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                                )
                            }
                        }
                    }
                    previewAlternates.forEach { choice ->
                        BranchPreviewCard(
                            choice = choice,
                            placement = placement,
                            onClick = { onOpenBranch(choice) },
                            onOpenContext = onOpenBranch,
                        )
                    }
                }

                if (placement == BranchPlacement.Before) {
                    BranchConnector(placement = placement, modifier = Modifier.padding(top = 26.dp))
                }
            }
        }
    }
}

private enum class BranchPlacement {
    Before,
    After,
}

@Composable
private fun BranchConnector(
    placement: BranchPlacement,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.width(14.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(3.dp),
    ) {
        Box(
            modifier = Modifier
                .width(3.dp)
                .height(24.dp)
                .clip(RoundedCornerShape(999.dp))
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.48f)),
        )
        Surface(
            shape = RoundedCornerShape(999.dp),
            color = if (placement == BranchPlacement.After) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.tertiary
            },
        ) {
            Box(
                modifier = Modifier
                    .width(10.dp)
                    .height(10.dp),
            )
        }
    }
}

@Composable
private fun BranchPreviewCard(
    choice: ThreadBranchChoice,
    placement: BranchPlacement,
    onClick: () -> Unit,
    onOpenContext: (ThreadBranchChoice) -> Unit,
) {
    val primaryColor = MaterialTheme.colorScheme.primary
    val highlightColor = MaterialTheme.colorScheme.tertiaryContainer
    val textSize = if (choice.isRecommended) {
        LocalNoteTextSize.current.value
    } else {
        (LocalNoteTextSize.current.value - 2).coerceAtLeast(11f)
    }
    val annotatedContent = remember(choice.note.content, primaryColor, highlightColor, textSize, choice.isRecommended) {
        buildMarkdownAnnotatedString(
            choice.note.content,
            primaryColor,
            highlightColor,
            textSize,
            isCompact = !choice.isRecommended,
        )
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .clickable(onClick = onClick),
        color = if (choice.isRecommended) {
            if (placement == BranchPlacement.After) {
                MaterialTheme.colorScheme.secondaryContainer
            } else {
                MaterialTheme.colorScheme.tertiaryContainer
            }
        } else {
            MaterialTheme.colorScheme.surfaceContainerHighest
        },
        shape = MaterialTheme.shapes.medium,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(
                    horizontal = if (choice.isRecommended) 16.dp else 14.dp,
                    vertical = if (choice.isRecommended) 15.dp else 14.dp,
                ),
            verticalArrangement = Arrangement.spacedBy(if (choice.isRecommended) 10.dp else 8.dp),
        ) {
            if (!choice.isRecommended) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "另一条",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        text = "约 ${choice.weight} 条",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Text(
                text = annotatedContent,
                maxLines = if (choice.isRecommended) Int.MAX_VALUE else 5,
                overflow = TextOverflow.Ellipsis,
                style = if (choice.isRecommended) {
                    MaterialTheme.typography.bodyLarge.copy(
                        fontFamily = LocalNoteFontFamily.current,
                        fontWeight = LocalNoteFontWeight.current,
                        fontSize = LocalNoteTextSize.current,
                        lineHeight = LocalNoteTextSize.current * LocalNoteLineSpacing.current,
                    )
                } else {
                    MaterialTheme.typography.bodyMedium.copy(
                        fontFamily = LocalNoteFontFamily.current,
                        fontWeight = LocalNoteFontWeight.current,
                    )
                },
            )
            if (!choice.isRecommended) {
                BranchContextQuotes(
                    parentChoices = choice.context.parents,
                    childChoices = choice.context.children,
                    onOpenBranch = onOpenContext,
                )
            } else {
                BranchContextQuotes(
                    parentChoices = emptyList(),
                    childChoices = choice.context.children,
                    onOpenBranch = onOpenContext,
                )
            }
        }
    }
}

@Composable
private fun BranchContextQuotes(
    parentChoices: List<ThreadBranchChoice>,
    childChoices: List<ThreadBranchChoice>,
    onOpenBranch: (ThreadBranchChoice) -> Unit,
) {
    val visibleParents = parentChoices.take(2)
    val visibleChildren = childChoices.take(2)
    if (visibleParents.isEmpty() && visibleChildren.isEmpty()) return

    Column(
        verticalArrangement = Arrangement.spacedBy(6.dp),
        modifier = Modifier.padding(top = 2.dp),
    ) {
        visibleParents.forEach { choice ->
            BranchContextQuote(
                label = "上游",
                choice = choice,
                onClick = { onOpenBranch(choice) },
            )
        }
        visibleChildren.forEach { choice ->
            BranchContextQuote(
                label = "下游",
                choice = choice,
                onClick = { onOpenBranch(choice) },
            )
        }
    }
}

@Composable
private fun BranchContextQuote(
    label: String,
    choice: ThreadBranchChoice,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.small)
            .clickable(onClick = onClick)
            .background(MaterialTheme.colorScheme.surfaceContainerLow)
            .padding(horizontal = 10.dp, vertical = 7.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .width(3.dp)
                .height(28.dp)
                .clip(RoundedCornerShape(999.dp))
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.62f)),
        )
        Column(
            verticalArrangement = Arrangement.spacedBy(2.dp),
            modifier = Modifier.weight(1f),
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            Text(
                text = choice.note.content.lineSequence().firstOrNull()?.trim().orEmpty().ifBlank { "空笔记" },
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun BranchChoiceSheetCard(
    choice: ThreadBranchChoice,
    onClick: () -> Unit,
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.large)
            .clickable(onClick = onClick),
        color = if (choice.isRecommended) {
            MaterialTheme.colorScheme.secondaryContainer
        } else {
            MaterialTheme.colorScheme.surfaceContainer
        },
        shape = MaterialTheme.shapes.large,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (choice.isRecommended) {
                    Surface(
                        color = MaterialTheme.colorScheme.primaryContainer,
                        shape = RoundedCornerShape(999.dp),
                    ) {
                        Text(
                            text = "建议先读",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onPrimaryContainer,
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                        )
                    }
                }
                Text(
                    text = "约 ${choice.weight} 条",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text(
                text = choice.note.content,
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 6,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}
