package com.synap.app.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlin.math.roundToInt

data class TimeClusterMarker(
    val id: String,
    val title: String,
    val rangeLabel: String,
    val noteCount: Int,
)

@Composable
fun TimeClusterSlider(
    markers: List<TimeClusterMarker>,
    collapsedAxisWeight: Float,
    hasOlder: Boolean,
    isLoadingOlder: Boolean,
    onLoadOlder: () -> Unit,
    modifier: Modifier = Modifier,
    hasNewer: Boolean = false,
    isLoadingNewer: Boolean = false,
    onLoadNewer: () -> Unit = {},
    onScrubbingChange: (Boolean) -> Unit = {},
    onScrubProgressChange: (Float) -> Unit = {},
) {
    if (markers.isEmpty()) {
        return
    }

    val axisLayout = remember(markers) { SessionAxisLayout.fromMarkers(markers) }
    val density = LocalDensity.current
    val lastIndex = axisLayout.lastSessionIndex
    val safeCollapsedWeight = collapsedAxisWeight.coerceIn(0f, axisLayout.totalWeight)
    val safeCollapsedProgress = axisLayout.weightToSessionProgress(safeCollapsedWeight)
    val safeSelectedIndex = axisLayout.weightToActiveSessionIndex(safeCollapsedWeight).coerceIn(0, lastIndex)
    val latestSelectedProgress by rememberUpdatedState(safeCollapsedProgress)
    val latestOnScrubbingChange by rememberUpdatedState(onScrubbingChange)
    val latestOnScrubProgressChange by rememberUpdatedState(onScrubProgressChange)

    var trackHeightPx by remember { mutableFloatStateOf(0f) }
    var isScrubbing by remember { mutableStateOf(false) }
    var dragAnchorIndex by remember { mutableFloatStateOf(0f) }
    var dragAnchorY by remember { mutableFloatStateOf(0f) }
    var currentTouchY by remember { mutableFloatStateOf(0f) }
    var dispatchedIndex by remember { mutableIntStateOf(safeSelectedIndex) }
    var lastOlderRequestCount by remember { mutableIntStateOf(-1) }
    var lastNewerRequestCount by remember { mutableIntStateOf(-1) }
    val animatedSliderWidth by animateDpAsState(
        targetValue = if (isScrubbing) 104.dp else 36.dp,
        label = "timeClusterSliderWidth",
    )
    val animatedTrackWidth by animateDpAsState(
        targetValue = if (isScrubbing) 48.dp else 28.dp,
        label = "timeClusterTrackWidth",
    )

    val stepPx = with(density) { 28.dp.toPx() }
    val edgeInsetPx = with(density) { 28.dp.toPx() }
    val tooltipHalfHeightPx = with(density) { 28.dp.roundToPx() }
    val railInsetPx = with(density) { 12.dp.toPx() }
    val baseRadiusPx = with(density) { 1.8.dp.toPx() }
    val expandedRadiusPx = with(density) { if (isScrubbing) 8.dp.toPx() else 4.5.dp.toPx() }
    val shiftPx = with(density) { if (isScrubbing) 22.dp.toPx() else 8.dp.toPx() }
    val fadePx = with(density) { 44.dp.toPx() }
    val collapsedStrokePx = with(density) { 1.25.dp.toPx() }
    val activeCollapsedStrokePx = with(density) { 2.dp.toPx() }
    val collapsedWeightStepPx = with(density) { 10.dp.toPx() }

    val rawFocusIndex = if (isScrubbing) {
        dragAnchorIndex + ((currentTouchY - dragAnchorY) / stepPx)
    } else {
        safeCollapsedProgress
    }
    val clampedFocusIndex = rawFocusIndex.coerceIn(0f, lastIndex.toFloat())
    val elasticOverscroll = rawFocusIndex - clampedFocusIndex
    val displayFocusIndex = clampedFocusIndex + elasticOverscroll * 0.32f
    val highlightedIndex = if (isScrubbing) dispatchedIndex else safeSelectedIndex
    val activeMarker = markers[highlightedIndex.coerceIn(0, lastIndex)]
    val sliderCenterY = if (trackHeightPx > 0f) trackHeightPx / 2f else edgeInsetPx * 4f
    val bubbleY = if (trackHeightPx <= 0f) {
        edgeInsetPx
    } else {
        val preferred = if (isScrubbing) {
            currentTouchY
        } else {
            sliderCenterY
        }
        preferred.coerceIn(edgeInsetPx, trackHeightPx - edgeInsetPx)
    }

    fun updateFromTouch(y: Float) {
        currentTouchY = y.coerceIn(0f, trackHeightPx.coerceAtLeast(y))
        val nextRawIndex = dragAnchorIndex + ((currentTouchY - dragAnchorY) / stepPx)
        val clampedProgress = nextRawIndex.coerceIn(0f, lastIndex.toFloat())
        dispatchedIndex = clampedProgress.roundToInt().coerceIn(0, lastIndex)
        latestOnScrubProgressChange(clampedProgress)
    }

    LaunchedEffect(safeSelectedIndex, isScrubbing) {
        if (!isScrubbing) {
            dispatchedIndex = safeSelectedIndex
        }
    }

    LaunchedEffect(markers.size, safeCollapsedProgress, hasOlder, isLoadingOlder, isScrubbing, clampedFocusIndex) {
        val needsBuffer = markers.size < 10
        val focusProgress = if (isScrubbing) clampedFocusIndex else safeCollapsedProgress
        val nearOlderEdge = focusProgress >= markers.lastIndex - 2f
        if (hasOlder && !isLoadingOlder && (needsBuffer || nearOlderEdge) && lastOlderRequestCount != markers.size) {
            lastOlderRequestCount = markers.size
            onLoadOlder()
        }
    }

    LaunchedEffect(markers.size, safeCollapsedProgress, hasNewer, isLoadingNewer, isScrubbing, clampedFocusIndex) {
        val focusProgress = if (isScrubbing) clampedFocusIndex else safeCollapsedProgress
        val nearNewerEdge = focusProgress <= 1f
        if (hasNewer && !isLoadingNewer && nearNewerEdge && lastNewerRequestCount != markers.size) {
            lastNewerRequestCount = markers.size
            onLoadNewer()
        }
    }

    Box(
        modifier = modifier.width(animatedSliderWidth),
    ) {
        AnimatedVisibility(
            visible = isScrubbing,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier
                .align(Alignment.TopStart)
                .offset {
                    IntOffset(
                        x = 0,
                        y = bubbleY.roundToInt() - tooltipHalfHeightPx,
                    )
                },
        ) {
            SliderTooltip(marker = activeMarker)
        }

        Box(
            modifier = Modifier
                .align(Alignment.CenterEnd)
                .width(animatedTrackWidth)
                .fillMaxHeight()
                .onSizeChanged { trackHeightPx = it.height.toFloat() }
                .pointerInput(markers.size) {
                    awaitEachGesture {
                        val down = awaitFirstDown(requireUnconsumed = false)
                        isScrubbing = true
                        latestOnScrubbingChange(true)
                        dragAnchorIndex = latestSelectedProgress
                        dragAnchorY = down.position.y
                        updateFromTouch(down.position.y)
                        down.consume()

                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull { it.id == down.id } ?: break
                            if (!change.pressed) {
                                break
                            }

                            updateFromTouch(change.position.y)
                            change.consume()
                        }

                        isScrubbing = false
                        latestOnScrubbingChange(false)
                    }
                },
        ) {
            val dotColor = MaterialTheme.colorScheme.onSurfaceVariant
            val activeDotColor = MaterialTheme.colorScheme.primary
            val collapsedLineColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.26f)

            Canvas(modifier = Modifier.fillMaxSize()) {
                val canvasHeight = size.height
                val railX = size.width - railInsetPx

                if (!isScrubbing) {
                    val boundaryMarkers = buildList {
                        for (boundaryIndex in 0..axisLayout.sessionCount) {
                            val boundaryWeight = axisLayout.boundaryWeight(boundaryIndex)
                            val y = sliderCenterY + ((boundaryWeight - safeCollapsedWeight) * collapsedWeightStepPx)
                            if (y >= -fadePx && y <= canvasHeight + fadePx) {
                                add(
                                    BoundaryMarker(
                                        boundaryIndex = boundaryIndex,
                                        y = y,
                                    ),
                                )
                            }
                        }
                    }
                    val activeSegmentIndex = axisLayout.weightToActiveSessionIndex(safeCollapsedWeight)

                    boundaryMarkers.zipWithNext().forEach { (start, end) ->
                        val edgeFade = minOf(edgeFadeForY(start.y, canvasHeight, fadePx), edgeFadeForY(end.y, canvasHeight, fadePx))
                        val isActiveSegment = start.boundaryIndex == activeSegmentIndex && end.boundaryIndex == activeSegmentIndex + 1
                        val color = if (isActiveSegment) activeDotColor else collapsedLineColor
                        drawLine(
                            color = color.copy(alpha = (if (isActiveSegment) 0.56f else 0.24f) * edgeFade),
                            start = Offset(railX, start.y),
                            end = Offset(railX, end.y),
                            strokeWidth = if (isActiveSegment) activeCollapsedStrokePx else collapsedStrokePx,
                            cap = StrokeCap.Round,
                        )
                    }

                    boundaryMarkers.forEach { marker ->
                        val edgeFade = edgeFadeForY(marker.y, canvasHeight, fadePx)
                        val isActiveBoundary = marker.boundaryIndex == activeSegmentIndex || marker.boundaryIndex == activeSegmentIndex + 1
                        drawCircle(
                            color = if (isActiveBoundary) activeDotColor else dotColor,
                            radius = baseRadiusPx,
                            center = Offset(railX, marker.y),
                            alpha = (if (isActiveBoundary) 0.92f else 0.48f) * edgeFade,
                        )
                    }
                } else {
                    val visibleMarkers = buildList {
                        markers.forEachIndexed { index, marker ->
                            val y = sliderCenterY + ((index - displayFocusIndex) * stepPx)
                            if (y >= -fadePx && y <= canvasHeight + fadePx) {
                                add(VisibleMarker(index = index, y = y, noteCount = marker.noteCount))
                            }
                        }
                    }

                    visibleMarkers.forEach { marker ->
                        val distance = abs(marker.index - displayFocusIndex)
                        val focus = smoothStep((1f - (distance / 3.4f)).coerceIn(0f, 1f))
                        val densityBoost = ((marker.noteCount.coerceAtMost(10) - 1).coerceAtLeast(0) / 9f)
                        val radius = baseRadiusPx + ((expandedRadiusPx + densityBoost * 2.dp.toPx()) - baseRadiusPx) * focus
                        val shift = shiftPx * focus * (0.72f + densityBoost * 0.28f)
                        val edgeFade = edgeFadeForY(marker.y, canvasHeight, fadePx)
                        val alpha = 0.18f + (0.88f * focus)
                        drawCircle(
                            color = if (marker.index == highlightedIndex) activeDotColor else dotColor,
                            radius = radius,
                            center = Offset(x = railX - shift, y = marker.y),
                            alpha = alpha * edgeFade,
                        )
                    }
                }
            }

            if (isLoadingOlder) {
                CircularProgressIndicator(
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = 10.dp)
                        .size(12.dp),
                    strokeWidth = 1.5.dp,
                )
            }

            if (isLoadingNewer) {
                CircularProgressIndicator(
                    modifier = Modifier
                        .align(Alignment.TopCenter)
                        .padding(top = 10.dp)
                        .size(12.dp),
                    strokeWidth = 1.5.dp,
                )
            }
        }
    }
}

private data class BoundaryMarker(
    val boundaryIndex: Int,
    val y: Float,
)

private data class VisibleMarker(
    val index: Int,
    val y: Float,
    val noteCount: Int,
)

private fun edgeFadeForY(
    y: Float,
    height: Float,
    fadePx: Float,
): Float = when {
    y < fadePx -> (y / fadePx).coerceIn(0f, 1f)
    y > height - fadePx -> ((height - y) / fadePx).coerceIn(0f, 1f)
    else -> 1f
}

@Composable
private fun SliderTooltip(
    marker: TimeClusterMarker,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier.border(
            width = 1.dp,
            color = MaterialTheme.colorScheme.outline.copy(alpha = 0.08f),
            shape = RoundedCornerShape(18.dp),
        ),
        shape = RoundedCornerShape(18.dp),
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 2.dp,
        shadowElevation = 8.dp,
    ) {
        Column(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            horizontalAlignment = Alignment.End,
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                text = marker.title,
                style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.SemiBold),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = "${marker.rangeLabel} · ${marker.noteCount} 条笔记",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

private fun smoothStep(value: Float): Float =
    value * value * (3f - 2f * value)
