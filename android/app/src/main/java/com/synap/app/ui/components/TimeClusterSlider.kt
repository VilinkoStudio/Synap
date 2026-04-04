package com.synap.app.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.ui.theme.MyApplicationTheme
import kotlin.math.abs
import kotlin.math.roundToInt

data class TimeClusterMarker(
    val title: String,
    val rangeLabel: String,
    val noteCount: Int,
    val previewText: String,
)

@Composable
fun CompactTimeClusterSlider(
    markers: List<TimeClusterMarker>,
    selectedIndex: Int,
    onSelectedIndexChange: (Int) -> Unit,
    expanded: Boolean,
    onExpandedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (markers.isEmpty()) {
        return
    }

    val density = LocalDensity.current
    val haptic = LocalHapticFeedback.current
    val safeSelectedIndex = selectedIndex.coerceIn(markers.indices)
    val activeMarker = markers[safeSelectedIndex]

    val railHeight = (28 + ((markers.size - 1).coerceAtLeast(0) * 18)).dp
    val railWidth by animateDpAsState(
        targetValue = if (expanded) 40.dp else 14.dp,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "time_slider_width",
    )
    val hostWidth by animateDpAsState(
        targetValue = if (expanded) 112.dp else 18.dp,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "time_slider_host_width",
    )
    val railColor by animateColorAsState(
        targetValue = if (expanded) {
            MaterialTheme.colorScheme.surface.copy(alpha = 0.96f)
        } else {
            Color.Transparent
        },
        label = "time_slider_rail_color",
    )
    val railBorderColor by animateColorAsState(
        targetValue = if (expanded) {
            MaterialTheme.colorScheme.outline.copy(alpha = 0.12f)
        } else {
            Color.Transparent
        },
        label = "time_slider_border_color",
    )

    val edgePaddingPx = with(density) { 12.dp.toPx() }
    val labelHeightPx = with(density) { 26.dp.toPx() }

    var trackHeightPx by remember { mutableFloatStateOf(0f) }
    var lastHapticIndex by rememberSaveable { mutableIntStateOf(safeSelectedIndex) }

    fun updateSelection(y: Float) {
        val nextIndex = markerIndexForPosition(
            y = y,
            totalHeight = trackHeightPx,
            markerCount = markers.size,
            edgePaddingPx = edgePaddingPx,
        )

        if (nextIndex != safeSelectedIndex) {
            onSelectedIndexChange(nextIndex)
            if (lastHapticIndex != nextIndex) {
                haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                lastHapticIndex = nextIndex
            }
        }
    }

    Box(
        modifier = modifier
            .width(hostWidth)
            .height(railHeight),
    ) {
        AnimatedVisibility(
            visible = expanded,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier
                .align(Alignment.TopStart)
                .offset {
                    val rawY = markerCenterOffset(
                        index = safeSelectedIndex,
                        markerCount = markers.size,
                        contentHeight = trackHeightPx,
                        edgePaddingPx = edgePaddingPx,
                    ) - (labelHeightPx / 2f)
                    val maxY = (trackHeightPx - labelHeightPx).coerceAtLeast(0f)
                    IntOffset(0, rawY.roundToInt().coerceIn(0, maxY.roundToInt()))
                },
        ) {
            SelectedTimeLabel(marker = activeMarker)
        }

        Surface(
            modifier = Modifier
                .align(Alignment.CenterEnd)
                .width(railWidth)
                .fillMaxHeight()
                .onSizeChanged { size -> trackHeightPx = size.height.toFloat() }
                .pointerInput(markers.size, trackHeightPx) {
                    awaitEachGesture {
                        val down = awaitFirstDown(requireUnconsumed = false)
                        onExpandedChange(true)
                        updateSelection(down.position.y)

                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull { it.id == down.id } ?: break
                            if (!change.pressed) {
                                break
                            }

                            updateSelection(change.position.y)
                            change.consume()
                        }

                        onExpandedChange(false)
                    }
                },
            shape = RoundedCornerShape(999.dp),
            color = railColor,
            border = BorderStroke(1.dp, railBorderColor),
            tonalElevation = if (expanded) 1.dp else 0.dp,
            shadowElevation = if (expanded) 4.dp else 0.dp,
        ) {
            Box(modifier = Modifier.fillMaxSize()) {
                if (expanded) {
                    Box(
                        modifier = Modifier
                            .align(Alignment.Center)
                            .width(1.dp)
                            .fillMaxHeight()
                            .padding(vertical = 10.dp)
                            .background(MaterialTheme.colorScheme.outline.copy(alpha = 0.14f)),
                    )
                }

                markers.forEachIndexed { index, _ ->
                    TimeClusterDot(
                        index = index,
                        markerCount = markers.size,
                        selectedIndex = safeSelectedIndex,
                        contentHeight = trackHeightPx,
                        edgePaddingPx = edgePaddingPx,
                        expanded = expanded,
                    )
                }
            }
        }
    }
}

@Composable
private fun BoxScope.TimeClusterDot(
    index: Int,
    markerCount: Int,
    selectedIndex: Int,
    contentHeight: Float,
    edgePaddingPx: Float,
    expanded: Boolean,
) {
    val distance = abs(index - selectedIndex)
    val visible = expanded || distance <= 2
    val density = LocalDensity.current

    val dotSize by animateDpAsState(
        targetValue = when {
            !visible -> 1.dp
            distance == 0 && expanded -> 7.dp
            distance == 0 -> 5.dp
            distance == 1 && expanded -> 4.dp
            distance == 1 -> 3.dp
            else -> 2.dp
        },
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "time_slider_dot_size",
    )
    val dotAlpha by animateFloatAsState(
        targetValue = when {
            !visible -> 0f
            distance == 0 -> 1f
            distance == 1 && expanded -> 0.42f
            distance == 1 -> 0.34f
            expanded -> 0.22f
            else -> 0.16f
        },
        animationSpec = spring(stiffness = Spring.StiffnessLow),
        label = "time_slider_dot_alpha",
    )
    val dotColor by animateColorAsState(
        targetValue = if (distance == 0) {
            MaterialTheme.colorScheme.primary
        } else {
            MaterialTheme.colorScheme.onSurface
        },
        label = "time_slider_dot_color",
    )
    val dotRadiusPx = with(density) { dotSize.toPx() / 2f }

    Box(
        modifier = Modifier
            .align(Alignment.TopCenter)
            .offset {
                val y = markerCenterOffset(
                    index = index,
                    markerCount = markerCount,
                    contentHeight = contentHeight,
                    edgePaddingPx = edgePaddingPx,
                ) - dotRadiusPx
                IntOffset(0, y.roundToInt())
            }
            .size(dotSize)
            .alpha(dotAlpha)
            .clip(CircleShape)
            .background(dotColor),
    )
}

@Composable
private fun SelectedTimeLabel(
    marker: TimeClusterMarker,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(999.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.95f),
        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
        tonalElevation = 0.dp,
        shadowElevation = 0.dp,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.08f)),
    ) {
        Text(
            text = "${marker.title} · ${marker.noteCount}",
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
            style = MaterialTheme.typography.labelMedium.copy(
                fontWeight = FontWeight.SemiBold,
                letterSpacing = 0.1.sp,
            ),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
fun TimeClusterSliderShowcase(
    startExpanded: Boolean,
    modifier: Modifier = Modifier,
) {
    val markers = remember {
        listOf(
            TimeClusterMarker(
                title = "此刻",
                rangeLabel = "00:20 - 02:10",
                noteCount = 3,
                previewText = "灵感和待办都集中在这一小段时间里。",
            ),
            TimeClusterMarker(
                title = "昨晚",
                rangeLabel = "21:10 - 23:40",
                noteCount = 6,
                previewText = "一口气写了几条复盘，回看时更像一个完整状态。",
            ),
            TimeClusterMarker(
                title = "周三午后",
                rangeLabel = "13:00 - 16:00",
                noteCount = 4,
                previewText = "会议记录、任务拆分和一条突然出现的想法混在一起。",
            ),
            TimeClusterMarker(
                title = "上周末",
                rangeLabel = "19:30 - 22:00",
                noteCount = 5,
                previewText = "生活记录和轻松的随手笔记堆成了一小段安静时间。",
            ),
            TimeClusterMarker(
                title = "三月下旬",
                rangeLabel = "03/21 - 03/28",
                noteCount = 9,
                previewText = "这个阶段写得很密，像是某个主题持续发酵之后的输出。",
            ),
        )
    }

    var selectedIndex by rememberSaveable { mutableIntStateOf(1) }
    var expanded by rememberSaveable { mutableStateOf(startExpanded) }

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .padding(horizontal = 20.dp, vertical = 24.dp),
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(end = 96.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(
                text = "时间线",
                style = MaterialTheme.typography.titleLarge.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.2.sp,
                ),
            )
            Text(
                text = if (expanded) {
                    "按住右侧时间点时，它会展开成一条很细的可滑动时间轨。"
                } else {
                    "平时只是一小列时间刻痕，轻一点，也更像页面本身的一部分。"
                },
                style = MaterialTheme.typography.bodyMedium.copy(
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.68f),
                    lineHeight = 20.sp,
                ),
            )

            Spacer(modifier = Modifier.height(6.dp))

            markers.forEachIndexed { index, marker ->
                TimeClusterSliceRow(
                    marker = marker,
                    isSelected = index == selectedIndex,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }

        CompactTimeClusterSlider(
            markers = markers,
            selectedIndex = selectedIndex,
            onSelectedIndexChange = { selectedIndex = it },
            expanded = expanded,
            onExpandedChange = { expanded = it },
            modifier = Modifier.align(Alignment.CenterEnd),
        )
    }
}

@Composable
private fun TimeClusterSliceRow(
    marker: TimeClusterMarker,
    isSelected: Boolean,
    modifier: Modifier = Modifier,
) {
    val containerColor by animateColorAsState(
        targetValue = if (isSelected) {
            MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.82f)
        } else {
            MaterialTheme.colorScheme.surface.copy(alpha = 0.72f)
        },
        label = "time_slice_row_color",
    )
    val stripeAlpha by animateFloatAsState(
        targetValue = if (isSelected) 1f else 0f,
        label = "time_slice_row_stripe_alpha",
    )

    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(18.dp),
        color = containerColor,
        tonalElevation = 0.dp,
        shadowElevation = 0.dp,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.06f)),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.Top,
        ) {
            Box(
                modifier = Modifier
                    .padding(top = 2.dp)
                    .width(2.dp)
                    .height(30.dp)
                    .clip(RoundedCornerShape(999.dp))
                    .background(MaterialTheme.colorScheme.primary.copy(alpha = stripeAlpha)),
            )

            Spacer(modifier = Modifier.width(12.dp))

            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = marker.title,
                        style = MaterialTheme.typography.titleMedium.copy(
                            fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Medium,
                        ),
                    )
                    Text(
                        text = "${marker.noteCount} 条",
                        style = MaterialTheme.typography.labelMedium.copy(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.58f),
                        ),
                    )
                }

                Text(
                    text = marker.previewText,
                    style = MaterialTheme.typography.bodyMedium.copy(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.72f),
                        lineHeight = 20.sp,
                    ),
                )

                Text(
                    text = marker.rangeLabel,
                    style = MaterialTheme.typography.labelMedium.copy(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.48f),
                    ),
                )
            }
        }
    }
}

private fun markerIndexForPosition(
    y: Float,
    totalHeight: Float,
    markerCount: Int,
    edgePaddingPx: Float,
): Int {
    if (markerCount <= 1 || totalHeight <= edgePaddingPx * 2f) {
        return 0
    }

    val top = edgePaddingPx
    val bottom = totalHeight - edgePaddingPx
    val step = (bottom - top) / (markerCount - 1)
    val clamped = y.coerceIn(top, bottom)
    return ((clamped - top) / step).roundToInt().coerceIn(0, markerCount - 1)
}

private fun markerCenterOffset(
    index: Int,
    markerCount: Int,
    contentHeight: Float,
    edgePaddingPx: Float,
): Float {
    if (markerCount <= 1 || contentHeight <= edgePaddingPx * 2f) {
        return contentHeight / 2f
    }

    val top = edgePaddingPx
    val bottom = contentHeight - edgePaddingPx
    val fraction = index / (markerCount - 1).toFloat()
    return top + (bottom - top) * fraction
}

@Preview(name = "Time Slider Collapsed", widthDp = 430, heightDp = 920, showBackground = true)
@Composable
private fun TimeClusterSliderCollapsedPreview() {
    MyApplicationTheme(dynamicColor = false) {
        TimeClusterSliderShowcase(startExpanded = false)
    }
}

@Preview(name = "Time Slider Expanded", widthDp = 430, heightDp = 920, showBackground = true)
@Composable
private fun TimeClusterSliderExpandedPreview() {
    MyApplicationTheme(dynamicColor = false) {
        TimeClusterSliderShowcase(startExpanded = true)
    }
}
