package com.synap.app.ui.screens

import android.content.Context
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Tag
import androidx.compose.material.icons.filled.ViewAgenda
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.SegmentedSettingsColumn
import com.synap.app.ui.components.SegmentedSwitchItem
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingHomeScreen(
    showTagBar: Boolean,
    showTimeGroups: Boolean,
    showTimelineJumpTool: Boolean,
    onSetHomeDisplayOptions: (Boolean, Boolean, Boolean) -> Unit,
    onRefresh: () -> Unit,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("synap_prefs", Context.MODE_PRIVATE) }

    var isNavCollapsed by remember { mutableStateOf(prefs.getBoolean("is_nav_collapsed", false)) }
    var dualColumnCards by remember { mutableStateOf(prefs.getBoolean("dual_column_cards", true)) }
    var tagBarEnabled by remember(showTagBar) { mutableStateOf(showTagBar) }
    var timeGroupsEnabled by remember(showTimeGroups) { mutableStateOf(showTimeGroups) }
    var timelineJumpToolEnabled by remember(showTimelineJumpTool) { mutableStateOf(showTimelineJumpTool) }

    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent -> backProgress = backEvent.progress }
            onNavigateBack()
        } catch (e: CancellationException) {
            backProgress = 0f
        }
    }

    fun updateHomeDisplayOptions(
        nextTagBarEnabled: Boolean = tagBarEnabled,
        nextTimeGroupsEnabled: Boolean = timeGroupsEnabled,
        nextTimelineJumpToolEnabled: Boolean = timelineJumpToolEnabled,
    ) {
        tagBarEnabled = nextTagBarEnabled
        timeGroupsEnabled = nextTimeGroupsEnabled
        timelineJumpToolEnabled = nextTimelineJumpToolEnabled
        onSetHomeDisplayOptions(nextTagBarEnabled, nextTimeGroupsEnabled, nextTimelineJumpToolEnabled)
        onRefresh()
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            .graphicsLayer {
                translationX = backProgress * 64.dp.toPx()
                transformOrigin = TransformOrigin(1f, 0.5f)
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            },
        contentWindowInsets = WindowInsets(0, 0, 0, 0),
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.setting_home_layout)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp)
        ) {
            SegmentedSettingsColumn(title = stringResource(R.string.setting_home_display_options)) {
                item {
                    SegmentedSwitchItem(
                        icon = Icons.Filled.Tag,
                        title = stringResource(R.string.setting_show_tag_bar),
                        checked = tagBarEnabled,
                        onCheckedChange = { updateHomeDisplayOptions(nextTagBarEnabled = it) },
                    )
                }
                item {
                    SegmentedSwitchItem(
                        icon = Icons.Filled.ViewAgenda,
                        title = stringResource(R.string.setting_show_time_groups),
                        checked = timeGroupsEnabled,
                        onCheckedChange = { updateHomeDisplayOptions(nextTimeGroupsEnabled = it) },
                    )
                }
                item {
                    SegmentedSwitchItem(
                        icon = Icons.Filled.CalendarMonth,
                        title = stringResource(R.string.setting_show_timeline_jump_tool),
                        checked = timelineJumpToolEnabled,
                        onCheckedChange = { updateHomeDisplayOptions(nextTimelineJumpToolEnabled = it) },
                    )
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            SegmentedSettingsColumn(title = stringResource(R.string.setting_phone_layout)) {
                item {
                    SegmentedSwitchItem(
                        icon = Icons.Filled.MoreVert,
                        title = stringResource(R.string.setting_collapse_nav_buttons),
                        checked = isNavCollapsed,
                        onCheckedChange = {
                            isNavCollapsed = it
                            prefs.edit().putBoolean("is_nav_collapsed", it).apply()
                        },
                    )
                }
                item {
                    SegmentedSwitchItem(
                        icon = Icons.Filled.ViewAgenda,
                        title = stringResource(R.string.setting_dual_column_cards),
                        checked = dualColumnCards,
                        onCheckedChange = {
                            dualColumnCards = it
                            prefs.edit().putBoolean("dual_column_cards", it).apply()
                        },
                    )
                }
            }

            Spacer(modifier = Modifier.height(32.dp))
        }
    }
}
