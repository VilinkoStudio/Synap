package com.synap.app.ui.screens
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ZoomOutMap
import androidx.compose.material3.AssistChip
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.data.model.StarmapPointRecord
import com.synap.app.ui.viewmodel.StarmapNoteSnapshot
import com.synap.app.ui.viewmodel.StarmapUiState
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlin.math.ln
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.max
import java.util.concurrent.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StarmapScreen(
    uiState: StarmapUiState,
    onNavigateBack: () -> Unit,
    onRefresh: () -> Unit,
) {
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
                title = { Text(stringResource(R.string.starmap_title)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    TextButton(onClick = onRefresh) {
                        Text(stringResource(R.string.retry))
                    }
                },
            )
        },
    ) { innerPadding ->
        when {
            uiState.isLoading && uiState.points.isEmpty() -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding),
                    contentAlignment = Alignment.Center,
                ) {
                    CircularProgressIndicator()
                }
            }

            uiState.errorMessage != null && uiState.points.isEmpty() -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding),
                    contentAlignment = Alignment.Center,
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text(
                            text = uiState.errorMessage,
                            color = MaterialTheme.colorScheme.error,
                        )
                        TextButton(onClick = onRefresh) {
                            Text(stringResource(R.string.retry))
                        }
                    }
                }
            }

            else -> {
                StarmapUniverse(
                    points = uiState.points,
                    noteSnapshots = uiState.noteSnapshots,
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding)
                        .padding(16.dp)
                )
            }
        }
    }
}

@Composable
private fun StarmapUniverse(
    points: List<StarmapPointRecord>,
    noteSnapshots: Map<String, StarmapNoteSnapshot>,
    modifier: Modifier = Modifier,
) {
    var scale by rememberSaveable { mutableFloatStateOf(1f) }
    var translationX by rememberSaveable { mutableFloatStateOf(0f) }
    var translationY by rememberSaveable { mutableFloatStateOf(0f) }
    var canvasSize by remember { mutableStateOf(IntSize.Zero) }

    val density = LocalDensity.current
    val pointRadiusPx = with(density) { 4.dp.toPx() }
    val axisStrokePx = with(density) { 1.dp.toPx() }
    val worldSpreadPx = with(density) { 180.dp.toPx() }
    val minScale = 0.08f
    val maxScale = 120f

    val cardColor = MaterialTheme.colorScheme.surfaceContainerHigh
    val accentColor = MaterialTheme.colorScheme.primary
    val axisColor = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.7f)
    val gridColor = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.22f)
    val coolColor = lerp(
        MaterialTheme.colorScheme.primary,
        MaterialTheme.colorScheme.tertiary,
        0.55f,
    ).copy(alpha = 0.98f)
    val warmColor = lerp(
        MaterialTheme.colorScheme.secondary,
        MaterialTheme.colorScheme.error,
        0.42f,
    ).copy(alpha = 0.98f)
    val labelCellWidthPx = with(density) { 188.dp.toPx() }
    val labelCellHeightPx = with(density) { 104.dp.toPx() }
    val labelWidthDp = 208.dp
    val labelMinHeightDp = 56.dp
    val labelMaxHeightDp = 128.dp
    val labelOffsetXPx = with(density) { 10.dp.toPx() }
    val labelOffsetYPx = with(density) { 14.dp.toPx() }

    Box(
        modifier = modifier
            .clip(RoundedCornerShape(28.dp))
            .background(
                brush = Brush.radialGradient(
                    colors = listOf(
                        MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.28f),
                        MaterialTheme.colorScheme.surfaceContainerLowest,
                        MaterialTheme.colorScheme.surface,
                    ),
                ),
            )
            .onSizeChanged { canvasSize = it }
            .pointerInput(Unit) {
                detectDragGestures { change, dragAmount ->
                    change.consume()
                    translationX += dragAmount.x
                    translationY += dragAmount.y
                }
            }
            .pointerInput(Unit) {
                detectTransformGestures { centroid, pan, zoom, _ ->
                    val oldScale = scale
                    val newScale = (oldScale * zoom).coerceIn(minScale, maxScale)
                    val scaleRatio = if (oldScale == 0f) 1f else newScale / oldScale
                    val viewportCenter = Offset(
                        x = canvasSize.width / 2f,
                        y = canvasSize.height / 2f,
                    )

                    translationX =
                        (pan.x + centroid.x - viewportCenter.x) -
                            ((centroid.x - viewportCenter.x) - translationX) * scaleRatio
                    translationY =
                        (pan.y + centroid.y - viewportCenter.y) -
                            ((centroid.y - viewportCenter.y) - translationY) * scaleRatio
                    scale = newScale
                }
            },
    ) {
        val projectedPoints = remember(points, canvasSize, scale, translationX, translationY, worldSpreadPx) {
            projectPoints(
                points = points,
                noteSnapshots = noteSnapshots,
                canvasSize = canvasSize,
                scale = scale,
                translationX = translationX,
                translationY = translationY,
                worldSpreadPx = worldSpreadPx,
            )
        }
        val labelCandidates = remember(
            projectedPoints,
            canvasSize,
            labelCellWidthPx,
            labelCellHeightPx,
            scale,
            worldSpreadPx,
        ) {
            selectLabelCandidates(
                projectedPoints = projectedPoints,
                canvasSize = canvasSize,
                cellWidthPx = labelCellWidthPx,
                cellHeightPx = labelCellHeightPx,
                scale = scale,
                worldSpreadPx = worldSpreadPx,
            )
        }
        val labeledPointIds = remember(labelCandidates) {
            labelCandidates.map { it.point.id }.toHashSet()
        }
        val displayPoints = remember(projectedPoints, labeledPointIds) {
            projectedPoints.map { projected ->
                projected.copy(showLabel = projected.point.id in labeledPointIds)
            }
        }
        val timeRange = remember(noteSnapshots) {
            noteSnapshots.values
                .map { it.createdAt }
                .takeIf { it.isNotEmpty() }
                ?.let { timestamps -> timestamps.minOrNull()!! to timestamps.maxOrNull()!! }
        }

        Canvas(modifier = Modifier.fillMaxSize()) {
            val center = Offset(
                x = size.width / 2f,
                y = size.height / 2f,
            )
            val originX = center.x + translationX
            val originY = center.y + translationY
            val baseGridStep = (worldSpreadPx / 2f) * scale
            var gridStep = baseGridStep

            while (gridStep in 0.0001f..24f) {
                gridStep *= 2f
            }

            if (gridStep.isFinite() && gridStep > 0f) {
                var x = originX % gridStep
                if (x < 0f) x += gridStep
                while (x <= size.width) {
                    drawLine(
                        color = gridColor,
                        start = Offset(x, 0f),
                        end = Offset(x, size.height),
                        strokeWidth = axisStrokePx,
                    )
                    x += gridStep
                }

                var y = originY % gridStep
                if (y < 0f) y += gridStep
                while (y <= size.height) {
                    drawLine(
                        color = gridColor,
                        start = Offset(0f, y),
                        end = Offset(size.width, y),
                        strokeWidth = axisStrokePx,
                    )
                    y += gridStep
                }
            }

            drawLine(
                color = axisColor,
                start = Offset(originX, 0f),
                end = Offset(originX, size.height),
                strokeWidth = axisStrokePx * 2f,
            )
            drawLine(
                color = axisColor,
                start = Offset(0f, originY),
                end = Offset(size.width, originY),
                strokeWidth = axisStrokePx * 2f,
            )

            displayPoints.forEach { projected ->
                val radius = if (projected.showLabel) pointRadiusPx else pointRadiusPx * 0.72f
                val glowRadius = if (projected.showLabel) pointRadiusPx * 3.2f else pointRadiusPx * 1.8f
                val pointColor = colorForTimestamp(
                    createdAt = projected.createdAt,
                    timeRange = timeRange,
                    coolColor = coolColor,
                    warmColor = warmColor,
                )
                val pointGlow = pointColor.copy(alpha = if (projected.showLabel) 0.30f else 0.18f)

                drawCircle(
                    color = pointGlow,
                    radius = glowRadius,
                    center = Offset(projected.screenX, projected.screenY),
                )
                drawCircle(
                    color = pointColor,
                    radius = radius,
                    center = Offset(projected.screenX, projected.screenY),
                )
            }
        }

        LabelOverlay(
            labels = labelCandidates,
            noteSnapshots = noteSnapshots,
            labelWidth = labelWidthDp,
            labelMinHeight = labelMinHeightDp,
            labelMaxHeight = labelMaxHeightDp,
            labelOffsetXPx = labelOffsetXPx,
            labelOffsetYPx = labelOffsetYPx,
        )

        Surface(
            modifier = Modifier
                .align(Alignment.TopStart)
                .padding(16.dp),
            shape = RoundedCornerShape(22.dp),
            color = cardColor.copy(alpha = 0.92f),
            tonalElevation = 6.dp,
            shadowElevation = 2.dp,
        ) {
            Column(modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp)) {
                Text(
                    text = stringResource(R.string.starmap_title),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = "点位 ${points.size}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(modifier = Modifier.height(10.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    AssistChip(
                        onClick = {},
                        enabled = false,
                        label = { Text("缩放 ${"%.2f".format(scale)}x") },
                        leadingIcon = {
                            Icon(
                                Icons.Filled.ZoomOutMap,
                                contentDescription = null,
                                modifier = Modifier.size(18.dp),
                            )
                        },
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    if (points.isEmpty()) {
                        Surface(
                            shape = CircleShape,
                            color = MaterialTheme.colorScheme.secondaryContainer,
                        ) {
                            Text(
                                text = "空",
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSecondaryContainer,
                            )
                        }
                    }
                }
            }
        }

        Surface(
            modifier = Modifier
                .align(Alignment.BottomStart)
                .padding(16.dp),
            shape = RoundedCornerShape(20.dp),
            color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.88f),
            tonalElevation = 4.dp,
        ) {
            Text(
                text = if (canvasSize == IntSize.Zero) {
                    "双指缩放，单指拖拽"
                } else {
                    "画布 ${max(canvasSize.width, 0)}×${max(canvasSize.height, 0)}  双指缩放，单指拖拽"
                },
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        if (points.isEmpty()) {
            Column(
                modifier = Modifier.align(Alignment.Center),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Surface(
                    color = accentColor.copy(alpha = 0.14f),
                    shape = CircleShape,
                ) {
                    Box(
                        modifier = Modifier.size(72.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Canvas(modifier = Modifier.size(24.dp)) {
                            drawCircle(color = accentColor, radius = size.minDimension / 2f)
                        }
                    }
                }
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = "暂时没有星图点位",
                    style = MaterialTheme.typography.titleMedium,
                )
            }
        }
    }
}

@Composable
private fun BoxScope.LabelOverlay(
    labels: List<ProjectedPoint>,
    noteSnapshots: Map<String, StarmapNoteSnapshot>,
    labelWidth: androidx.compose.ui.unit.Dp,
    labelMinHeight: androidx.compose.ui.unit.Dp,
    labelMaxHeight: androidx.compose.ui.unit.Dp,
    labelOffsetXPx: Float,
    labelOffsetYPx: Float,
) {
    labels.forEach { label ->
        val snapshot = noteSnapshots[label.point.id]
        val content = noteSnapshots[label.point.id]
            ?.content
            ?.replace('\n', ' ')
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?: "空白笔记"
        val timestampText = snapshot?.createdAt?.let(::formatStarmapTimestamp) ?: "未知时间"

        Surface(
            modifier = Modifier
                .offset {
                    IntOffset(
                        x = (label.screenX - labelOffsetXPx).roundToInt(),
                        y = (label.screenY - labelOffsetYPx).roundToInt(),
                    )
                }
                .widthIn(max = labelWidth)
                .heightIn(min = labelMinHeight, max = labelMaxHeight),
            shape = RoundedCornerShape(16.dp),
            color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.92f),
            tonalElevation = 4.dp,
            shadowElevation = 1.dp,
        ) {
            Column(modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp)) {
                Text(
                    text = content,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 4,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = timestampText,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

private fun formatStarmapTimestamp(timestampMs: Long): String {
    val formatter = SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.getDefault())
    return formatter.format(Date(timestampMs))
}

private data class ProjectedPoint(
    val point: StarmapPointRecord,
    val screenX: Float,
    val screenY: Float,
    val weight: Float,
    val title: String,
    val createdAt: Long?,
    val showLabel: Boolean,
)

private fun projectPoints(
    points: List<StarmapPointRecord>,
    noteSnapshots: Map<String, StarmapNoteSnapshot>,
    canvasSize: IntSize,
    scale: Float,
    translationX: Float,
    translationY: Float,
    worldSpreadPx: Float,
): List<ProjectedPoint> {
    if (canvasSize == IntSize.Zero) {
        return emptyList()
    }

    val centerX = canvasSize.width / 2f + translationX
    val centerY = canvasSize.height / 2f + translationY

    return points.map { point ->
        val screenX = centerX + point.x * worldSpreadPx * scale
        val screenY = centerY - point.y * worldSpreadPx * scale
        ProjectedPoint(
            point = point,
            screenX = screenX,
            screenY = screenY,
            weight = pointWeight(point),
            title = point.id.take(8),
            createdAt = noteSnapshots[point.id]?.createdAt,
            showLabel = false,
        )
    }
}

private fun selectLabelCandidates(
    projectedPoints: List<ProjectedPoint>,
    canvasSize: IntSize,
    cellWidthPx: Float,
    cellHeightPx: Float,
    scale: Float,
    worldSpreadPx: Float,
): List<ProjectedPoint> {
    if (canvasSize == IntSize.Zero) {
        return emptyList()
    }

    val effectiveScale = scale.coerceAtLeast(0.0001f)
    val worldCellWidth = cellWidthPx / (worldSpreadPx * effectiveScale)
    val worldCellHeight = cellHeightPx / (worldSpreadPx * effectiveScale)
    val visibleMarginX = cellWidthPx
    val visibleMarginY = cellHeightPx
    val occupiedCells = mutableSetOf<Pair<Int, Int>>()
    val sortedCandidates = projectedPoints
        .asSequence()
        .filter { projected ->
            projected.screenX >= -visibleMarginX &&
                projected.screenX <= canvasSize.width + visibleMarginX &&
                projected.screenY >= -visibleMarginY &&
                projected.screenY <= canvasSize.height + visibleMarginY
        }
        .sortedByDescending { it.weight }
        .toList()
    val winners = mutableListOf<ProjectedPoint>()
    val occupiedColsToRight = 1
    val occupiedRowsAbove = 1
    val occupiedRowsBelow = 0

    sortedCandidates.forEach { projected ->
        val col = kotlin.math.floor(projected.point.x / worldCellWidth).toInt()
        val row = kotlin.math.floor(projected.point.y / worldCellHeight).toInt()
        var blocked = false

        for (dx in 0..occupiedColsToRight) {
            for (dy in -occupiedRowsAbove..occupiedRowsBelow) {
                if ((col + dx) to (row + dy) in occupiedCells) {
                    blocked = true
                    break
                }
            }
            if (blocked) {
                break
            }
        }

        if (!blocked) {
            winners += projected
            for (dx in 0..occupiedColsToRight) {
                for (dy in -occupiedRowsAbove..occupiedRowsBelow) {
                    occupiedCells += (col + dx) to (row + dy)
                }
            }
        }
    }

    val winnerIds = winners.map { it.point.id }.toHashSet()
    return projectedPoints
        .map { projected ->
            projected.copy(showLabel = projected.point.id in winnerIds)
        }
        .filter { it.showLabel }
}

private fun pointWeight(point: StarmapPointRecord): Float {
    val radialBias = 1f / (1f + (point.x * point.x + point.y * point.y))
    return radialBias * 1000f + point.id.length
}

private fun colorForTimestamp(
    createdAt: Long?,
    timeRange: Pair<Long, Long>?,
    coolColor: Color,
    warmColor: Color,
): Color {
    val timestamp = createdAt ?: return coolColor
    val range = timeRange ?: return coolColor
    val minTime = range.first
    val maxTime = range.second
    if (maxTime <= minTime) {
        return warmColor
    }

    val age = (maxTime - timestamp).coerceAtLeast(0L).toDouble()
    val maxAge = (maxTime - minTime).coerceAtLeast(1L).toDouble()
    val logAge = ln(age + 1.0)
    val logMaxAge = ln(maxAge + 1.0)
    val normalizedAge = if (logMaxAge <= 0.0) 0.0 else (logAge / logMaxAge).coerceIn(0.0, 1.0)
    val recency = (1.0 - normalizedAge).toFloat().pow(0.55f)

    return lerp(coolColor, warmColor, recency)
}
