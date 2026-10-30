@file:OptIn(ExperimentalMaterial3ExpressiveApi::class)

package com.synap.app.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemColors
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.ListItemShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SegmentedListItem
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp

internal val LocalSegmentedShapes = compositionLocalOf<ListItemShapes?> { null }

@DslMarker
annotation class SegmentedSettingsDsl

@SegmentedSettingsDsl
class SegmentedSettingsScope {
    internal data class Entry(val content: @Composable () -> Unit)

    internal val entries = mutableListOf<Entry>()

    fun item(content: @Composable () -> Unit) {
        entries.add(Entry(content))
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SegmentedSettingsColumn(
    modifier: Modifier = Modifier,
    title: String? = null,
    content: SegmentedSettingsScope.() -> Unit,
) {
    val entries = SegmentedSettingsScope().apply(content).entries
    if (entries.isEmpty()) return

    Column(modifier = modifier) {
        if (title != null) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            Spacer(modifier = Modifier.height(12.dp))
        }
        Column(verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
            val singleShapes = if (entries.size == 1) {
                ListItemDefaults.shapes().copy(shape = MaterialTheme.shapes.large)
            } else {
                null
            }
            entries.forEachIndexed { index, entry ->
                CompositionLocalProvider(
                    LocalSegmentedShapes provides (
                        singleShapes ?: ListItemDefaults.segmentedShapes(index, entries.size)
                    )
                ) {
                    entry.content()
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun currentShapes(): ListItemShapes =
    LocalSegmentedShapes.current ?: ListItemDefaults.segmentedShapes(0, 1)

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun segmentedItemColors(): ListItemColors =
    ListItemDefaults.segmentedColors(
        containerColor = MaterialTheme.colorScheme.surfaceVariant,
        contentColor = MaterialTheme.colorScheme.onSurface,
        supportingContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
    )

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SegmentedNavItem(
    icon: ImageVector?,
    title: String,
    summary: String? = null,
    badge: String? = null,
    onClick: () -> Unit,
) {
    SegmentedListItem(
        onClick = onClick,
        shapes = currentShapes(),
        colors = segmentedItemColors(),
        verticalAlignment = Alignment.CenterVertically,
        leadingContent = icon?.let {
            { Icon(it, contentDescription = null, tint = MaterialTheme.colorScheme.primary) }
        },
        supportingContent = summary?.let { { Text(it) } },
        trailingContent = {
            Icon(
                Icons.AutoMirrored.Filled.KeyboardArrowRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        },
        content = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(title)
                if (badge != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Surface(
                        color = MaterialTheme.colorScheme.primary,
                        shape = RoundedCornerShape(4.dp),
                    ) {
                        Text(
                            text = badge,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onPrimary,
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                        )
                    }
                }
            }
        },
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SegmentedSwitchItem(
    icon: ImageVector?,
    title: String,
    summary: String? = null,
    checked: Boolean,
    enabled: Boolean = true,
    onCheckedChange: (Boolean) -> Unit,
) {
    SegmentedListItem(
        onClick = { onCheckedChange(!checked) },
        enabled = enabled,
        shapes = currentShapes(),
        colors = segmentedItemColors(),
        verticalAlignment = Alignment.CenterVertically,
        leadingContent = icon?.let {
            { Icon(it, contentDescription = null, tint = MaterialTheme.colorScheme.primary) }
        },
        supportingContent = summary?.let { { Text(it) } },
        trailingContent = {
            Switch(
                checked = checked,
                enabled = enabled,
                onCheckedChange = null,
                thumbContent = {
                    Icon(
                        imageVector = if (checked) Icons.Filled.Check else Icons.Filled.Close,
                        contentDescription = null,
                        modifier = Modifier.size(SwitchDefaults.IconSize),
                    )
                },
            )
        },
        content = { Text(title) },
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SegmentedSelectableItem(
    title: String,
    summary: String? = null,
    selected: Boolean,
    icon: ImageVector? = null,
    indent: Boolean = false,
    enabled: Boolean = true,
    titleStyle: TextStyle? = null,
    onClick: () -> Unit,
) {
    SegmentedListItem(
        selected = selected,
        onClick = onClick,
        enabled = enabled,
        shapes = currentShapes(),
        colors = segmentedItemColors(),
        verticalAlignment = Alignment.CenterVertically,
        leadingContent = when {
            indent -> { { Spacer(modifier = Modifier.size(24.dp)) } }
            icon != null -> { { Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.primary) } }
            else -> null
        },
        supportingContent = summary?.let { { Text(it) } },
        trailingContent = {
            if (selected) {
                Icon(
                    Icons.Filled.Check,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
        },
        content = { Text(text = title, style = titleStyle ?: MaterialTheme.typography.bodyLarge) },
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SegmentedInfoItem(
    icon: ImageVector?,
    title: String,
    summary: String? = null,
    titleStyle: TextStyle? = null,
    trailingContent: (@Composable () -> Unit)? = null,
) {
    SegmentedListItem(
        onClick = {},
        shapes = currentShapes(),
        colors = segmentedItemColors(),
        verticalAlignment = Alignment.CenterVertically,
        leadingContent = icon?.let {
            { Icon(it, contentDescription = null, tint = MaterialTheme.colorScheme.primary) }
        },
        supportingContent = summary?.let { { Text(it) } },
        trailingContent = trailingContent ?: {},
        content = { Text(text = title, style = titleStyle ?: MaterialTheme.typography.bodyLarge) },
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SegmentedContentItem(content: @Composable () -> Unit) {
    SegmentedListItem(
        onClick = {},
        shapes = currentShapes(),
        colors = segmentedItemColors(),
        content = { content() },
    )
}
