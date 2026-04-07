package com.synap.app.ui.components

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridItemSpan
import androidx.compose.foundation.lazy.staggeredgrid.itemsIndexed
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridItemInfo
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.TimelineSessionGroup
import com.synap.app.ui.util.formatSessionDayLabel
import com.synap.app.ui.util.formatSessionTimeRangeCompact
import com.synap.app.ui.util.formatSessionTimeRange
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlin.math.abs

@Composable
fun HomeSessionFeed(
    sessions: List<TimelineSessionGroup>,
    state: LazyStaggeredGridState,
    hasMore: Boolean,
    isLoadingMore: Boolean,
    onLoadMore: () -> Unit,
    isSelectionMode: Boolean, // 新增参数
    selectedNoteIds: Set<String>, // 新增参数
    onToggleSelection: (String) -> Unit, // 新增参数
    onEnterSelectionMode: (String) -> Unit, // 新增参数
    onOpenNote: (String) -> Unit,
    onToggleDeleted: (Note) -> Unit,
    onReplyToNote: (String, String) -> Unit,
) {
    val sessionHeaderIndexes = remember(sessions) { buildSessionHeaderIndexes(sessions) }
    val markers = remember(sessions) {
        sessions.map { session ->
            TimeClusterMarker(
                id = "${session.startedAt}_${session.endedAt}",
                title = formatSessionDayLabel(session.startedAt, session.endedAt),
                rangeLabel = formatSessionTimeRangeCompact(session.startedAt, session.endedAt),
                noteCount = session.noteCount,
            )
        }
    }
    val axisLayout = remember(markers) { SessionAxisLayout.fromMarkers(markers) }

    var isSliderScrubbing by remember { mutableStateOf(false) }
    var currentAxisWeight by remember { mutableStateOf(0f) }
    var collapsedAxisWeight by remember { mutableStateOf(0f) }
    var scrubTargetWeight by remember { mutableStateOf(0f) }
    val latestCurrentAxisWeight by rememberUpdatedState(currentAxisWeight)
    val latestScrubTargetWeight by rememberUpdatedState(scrubTargetWeight)
    val animatedEndPadding by animateDpAsState(
        targetValue = if (isSliderScrubbing) 92.dp else 40.dp,
        label = "sessionFeedEndPadding",
    )
    val animatedGridScaleX by animateFloatAsState(
        targetValue = if (isSliderScrubbing) 0.94f else 1f,
        label = "sessionFeedScaleX",
    )
    val density = LocalDensity.current
    val sliderOffsetPx = with(density) { 6.dp.toPx() }

    LaunchedEffect(sessions.size) {
        if (sessions.isEmpty()) {
            currentAxisWeight = 0f
            collapsedAxisWeight = 0f
            scrubTargetWeight = 0f
        } else {
            currentAxisWeight = currentAxisWeight.coerceIn(0f, axisLayout.totalWeight)
            collapsedAxisWeight = collapsedAxisWeight.coerceIn(0f, axisLayout.totalWeight)
            scrubTargetWeight = scrubTargetWeight.coerceIn(0f, axisLayout.totalWeight)
        }
    }

    LaunchedEffect(state, sessions, sessionHeaderIndexes, axisLayout) {
        snapshotFlow { sessionAxisWeightForState(state, sessions, sessionHeaderIndexes, axisLayout) }
            .distinctUntilChanged()
            .collect { weight ->
                val safeWeight = weight.coerceIn(0f, axisLayout.totalWeight)
                currentAxisWeight = safeWeight
                if (!isSliderScrubbing && sessions.isNotEmpty()) {
                    collapsedAxisWeight = safeWeight
                }
            }
    }

    LaunchedEffect(isSliderScrubbing, sessions, sessionHeaderIndexes, axisLayout, state) {
        if (!isSliderScrubbing || sessions.isEmpty()) {
            return@LaunchedEffect
        }

        while (true) {
            val currentWeight = latestCurrentAxisWeight.coerceIn(0f, axisLayout.totalWeight)
            val targetWeight = latestScrubTargetWeight.coerceIn(0f, axisLayout.totalWeight)
            val weightError = targetWeight - currentWeight

            if (abs(weightError) > 0.001f) {
                val pixelsPerWeight = estimatePixelsPerWeight(
                    state = state,
                    sessions = sessions,
                    headerIndexes = sessionHeaderIndexes,
                    axisLayout = axisLayout,
                    currentWeight = currentWeight,
                )
                if (pixelsPerWeight > 0f) {
                    val maxStepPx = estimateMaxScrubStepPx(state).coerceAtLeast(96f)
                    val deltaPx = (weightError * pixelsPerWeight)
                        .coerceIn(-maxStepPx, maxStepPx)

                    if (abs(deltaPx) > 0.5f) {
                        state.dispatchRawDelta(deltaPx)
                    }
                }
            }

            withFrameNanos { }
        }
    }

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .fillMaxHeight(),
    ) {
        LazyVerticalStaggeredGrid(
            columns = StaggeredGridCells.Adaptive(minSize = 240.dp),
            state = state,
            modifier = Modifier
                .fillMaxSize()
                .graphicsLayer {
                    scaleX = animatedGridScaleX
                    translationX = if (isSliderScrubbing) -sliderOffsetPx else 0f
                    transformOrigin = TransformOrigin(0f, 0.5f)
                },
            contentPadding = PaddingValues(
                start = 16.dp,
                top = 8.dp,
                end = animatedEndPadding,
                bottom = 96.dp,
            ),
            verticalItemSpacing = 12.dp,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            sessions.forEachIndexed { sessionIndex, session ->
                item(span = StaggeredGridItemSpan.FullLine) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text(
                            text = formatSessionTimeRange(session.startedAt, session.endedAt),
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = "${session.noteCount} 条笔记",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                itemsIndexed(
                    items = session.notes,
                    key = { _, note -> "${session.startedAt}_${note.id}_${note.isDeleted}" },
                ) { noteIndex, note ->
                    NoteCardItem(
                        note = note,
                        onClick = {
                            if (isSelectionMode) onToggleSelection(note.id) else onOpenNote(note.id)
                        },
                        onLongClick = {
                            if (!isSelectionMode) onEnterSelectionMode(note.id)
                        },
                        isSelectionMode = isSelectionMode,
                        isSelected = selectedNoteIds.contains(note.id),
                        onToggleDeleted = { onToggleDeleted(note) },
                        onReply = { onReplyToNote(note.id, note.content) },
                        animationDelayMillis = ((sessionIndex + noteIndex).coerceAtMost(8)) * 35,
                    )
                }
            }
        }

        if (markers.isNotEmpty()) {
            TimeClusterSlider(
                markers = markers,
                collapsedAxisWeight = collapsedAxisWeight,
                hasOlder = hasMore,
                isLoadingOlder = isLoadingMore,
                onLoadOlder = onLoadMore,
                onScrubbingChange = { scrubbing ->
                    isSliderScrubbing = scrubbing
                    if (scrubbing) {
                        scrubTargetWeight = currentAxisWeight.coerceIn(0f, axisLayout.totalWeight)
                    }
                    if (!scrubbing && sessions.isNotEmpty()) {
                        collapsedAxisWeight = currentAxisWeight.coerceIn(0f, axisLayout.totalWeight)
                    }
                },
                onScrubProgressChange = { progress ->
                    scrubTargetWeight = axisLayout.sessionProgressToWeight(progress)
                        .coerceIn(0f, axisLayout.totalWeight)
                },
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .fillMaxHeight()
                    .padding(end = 4.dp, top = 12.dp, bottom = 12.dp),
            )
        }
    }
}

private fun buildSessionHeaderIndexes(sessions: List<TimelineSessionGroup>): List<Int> {
    var runningIndex = 0
    return buildList {
        sessions.forEach { session ->
            add(runningIndex)
            runningIndex += 1 + session.notes.size
        }
    }
}

private fun sessionAxisWeightForState(
    state: LazyStaggeredGridState,
    sessions: List<TimelineSessionGroup>,
    headerIndexes: List<Int>,
    axisLayout: SessionAxisLayout,
): Float {
    if (sessions.isEmpty() || headerIndexes.isEmpty()) {
        return 0f
    }

    val topItem = state.layoutInfo.visibleItemsInfo
        .minWithOrNull(compareBy<LazyStaggeredGridItemInfo> { it.offset.y }.thenBy { it.index })
        ?: return 0f

    val itemHeight = topItem.size.height.coerceAtLeast(1)
    val itemProgress = (-topItem.offset.y / itemHeight.toFloat()).coerceIn(0f, 1f)
    val overallItemPosition = topItem.index + itemProgress

    return sessionAxisWeightForItemPosition(
        itemPosition = overallItemPosition,
        sessions = sessions,
        headerIndexes = headerIndexes,
        axisLayout = axisLayout,
    )
}

private fun sessionAxisWeightForItemPosition(
    itemPosition: Float,
    sessions: List<TimelineSessionGroup>,
    headerIndexes: List<Int>,
    axisLayout: SessionAxisLayout,
): Float {
    if (sessions.isEmpty() || headerIndexes.isEmpty()) {
        return 0f
    }

    val currentSessionIndex = sessionIndexForItemPosition(itemPosition, headerIndexes)
    val currentHeaderIndex = headerIndexes[currentSessionIndex].toFloat()
    val sessionSpan = (sessions[currentSessionIndex].noteCount + 1).coerceAtLeast(1).toFloat()
    val sectionProgress = ((itemPosition - currentHeaderIndex) / sessionSpan).coerceIn(0f, 1f)

    return axisLayout.weightForSessionFraction(currentSessionIndex, sectionProgress)
}

private fun estimatePixelsPerWeight(
    state: LazyStaggeredGridState,
    sessions: List<TimelineSessionGroup>,
    headerIndexes: List<Int>,
    axisLayout: SessionAxisLayout,
    currentWeight: Float,
): Float {
    val visibleItems = state.layoutInfo.visibleItemsInfo
    if (visibleItems.isEmpty() || sessions.isEmpty() || headerIndexes.isEmpty()) {
        return 0f
    }

    val topItem = visibleItems
        .minWithOrNull(compareBy<LazyStaggeredGridItemInfo> { it.offset.y }.thenBy { it.index })
    val bottomItem = visibleItems
        .maxWithOrNull(compareBy<LazyStaggeredGridItemInfo> { it.offset.y + it.size.height }.thenBy { it.index })

    if (topItem != null && bottomItem != null && bottomItem.index > topItem.index) {
        val topY = topItem.offset.y + (topItem.size.height / 2f)
        val bottomY = bottomItem.offset.y + (bottomItem.size.height / 2f)
        val topWeight = sessionAxisWeightForItemPosition(
            itemPosition = topItem.index + 0.5f,
            sessions = sessions,
            headerIndexes = headerIndexes,
            axisLayout = axisLayout,
        )
        val bottomWeight = sessionAxisWeightForItemPosition(
            itemPosition = bottomItem.index + 0.5f,
            sessions = sessions,
            headerIndexes = headerIndexes,
            axisLayout = axisLayout,
        )
        val weightSpan = bottomWeight - topWeight
        val pixelSpan = bottomY - topY

        if (weightSpan > 0.001f && pixelSpan > 0f) {
            return pixelSpan / weightSpan
        }
    }

    val averageItemHeight = visibleItems
        .map { it.size.height.toFloat() }
        .average()
        .toFloat()
        .coerceAtLeast(1f)
    val currentSessionIndex = axisLayout.weightToActiveSessionIndex(currentWeight)
    val sessionSpan = (sessions[currentSessionIndex].noteCount + 1).coerceAtLeast(1).toFloat()
    val sessionWeight = axisLayout.segmentWeights[currentSessionIndex].coerceAtLeast(1f)

    return averageItemHeight * (sessionSpan / sessionWeight)
}

private fun estimateMaxScrubStepPx(state: LazyStaggeredGridState): Float {
    val visibleItems = state.layoutInfo.visibleItemsInfo
    if (visibleItems.isEmpty()) {
        return 0f
    }

    val top = visibleItems.minOf { it.offset.y.toFloat() }
    val bottom = visibleItems.maxOf { (it.offset.y + it.size.height).toFloat() }
    return ((bottom - top).coerceAtLeast(0f)) * 0.85f
}

private fun sessionIndexForItemIndex(
    itemIndex: Int,
    headerIndexes: List<Int>,
): Int {
    if (headerIndexes.isEmpty()) {
        return 0
    }

    val foundIndex = headerIndexes.binarySearch(itemIndex)
    return if (foundIndex >= 0) {
        foundIndex
    } else {
        (-foundIndex - 2).coerceIn(0, headerIndexes.lastIndex)
    }
}

private fun sessionIndexForItemPosition(
    itemPosition: Float,
    headerIndexes: List<Int>,
): Int {
    if (headerIndexes.isEmpty()) {
        return 0
    }

    val discreteIndex = itemPosition.toInt().coerceAtLeast(0)
    return sessionIndexForItemIndex(discreteIndex, headerIndexes)
}
