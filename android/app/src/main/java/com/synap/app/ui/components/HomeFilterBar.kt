package com.synap.app.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically

import androidx.compose.foundation.horizontalScroll

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.synap.app.R


@OptIn(ExperimentalLayoutApi::class)
@Composable
fun HomeFilterBar(
    isFilterPanelOpen: Boolean,
    isTagsExpanded: Boolean,
    allTags: List<String>,
    unselectedTags: Set<String>,
    isUntaggedUnselected: Boolean,
    onToggleFilterPanel: (Boolean) -> Unit,
    onToggleTagsExpanded: () -> Unit,
    onToggleAllTags: () -> Unit,
    onToggleUntaggedFilter: () -> Unit,
    onToggleTagFilter: (String) -> Unit,
) {
    val isAllSelected = unselectedTags.isEmpty() && !isUntaggedUnselected

    Column {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                FilterChip(
                    selected = isFilterPanelOpen,
                    onClick = { onToggleFilterPanel(!isFilterPanelOpen) },
                    label = { Text(if (isFilterPanelOpen) "筛选已开启" else "按标签筛选") },
                )
                if (!isFilterPanelOpen) {
                    Surface(
                        color = MaterialTheme.colorScheme.secondaryContainer,
                        shape = MaterialTheme.shapes.large,
                    ) {
                        Text(
                            text = "时间分组视图",
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSecondaryContainer,
                        )
                    }
                }
            }
        }

        AnimatedVisibility(
            visible = isFilterPanelOpen,
            enter = expandVertically() + fadeIn(),
            exit = shrinkVertically() + fadeOut(),
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp)
                    .animateContentSize(),
                verticalAlignment = if (isTagsExpanded) Alignment.Top else Alignment.CenterVertically,
            ) {
                Box(modifier = Modifier.weight(1f)) {
                    if (isTagsExpanded) {
                        FlowRow(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            FilterChip(
                                selected = isAllSelected,
                                onClick = onToggleAllTags,
                                label = { Text(stringResource(R.string.home_tagbar_all)) },
                            )

                            FilterChip(
                                selected = !isUntaggedUnselected,
                                onClick = onToggleUntaggedFilter,
                                label = { Text(stringResource(R.string.home_tagbar_none)) },
                            )

                            if (allTags.isNotEmpty()) {
                                Box(
                                    modifier = Modifier.height(32.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    VerticalDivider(
                                        modifier = Modifier
                                            .height(24.dp)
                                            .padding(horizontal = 4.dp),
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.3f),
                                    )
                                }
                            }

                            allTags.forEach { tag ->
                                val isSelected = tag !in unselectedTags
                                FilterChip(
                                    selected = isSelected,
                                    onClick = { onToggleTagFilter(tag) },
                                    label = { Text(tag) },
                                )
                            }
                        }
                    } else {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            FilterChip(
                                selected = isAllSelected,
                                onClick = onToggleAllTags,
                                label = { Text(stringResource(R.string.home_tagbar_all)) },
                            )

                            FilterChip(
                                selected = !isUntaggedUnselected,
                                onClick = onToggleUntaggedFilter,
                                label = { Text(stringResource(R.string.home_tagbar_none)) },
                            )

                            if (allTags.isNotEmpty()) {
                                Box(
                                    modifier = Modifier.height(32.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    VerticalDivider(
                                        modifier = Modifier
                                            .height(24.dp)
                                            .padding(horizontal = 4.dp),
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.3f),
                                    )
                                }
                            }

                            allTags.forEach { tag ->
                                val isSelected = tag !in unselectedTags
                                FilterChip(
                                    selected = isSelected,
                                    onClick = { onToggleTagFilter(tag) },
                                    label = { Text(tag) },
                                )
                            }
                        }
                    }
                }

                IconButton(
                    onClick = onToggleTagsExpanded,
                    modifier = Modifier
                        .padding(start = 4.dp)
                        .height(32.dp),
                ) {
                    Icon(
                        imageVector = if (isTagsExpanded) Icons.Filled.ExpandLess else Icons.Filled.ExpandMore,
                        contentDescription = if (isTagsExpanded) "收起" else "展开",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}
