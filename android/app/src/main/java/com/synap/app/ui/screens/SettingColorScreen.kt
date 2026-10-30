package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.SegmentedContentItem
import com.synap.app.ui.components.SegmentedSelectableItem
import com.synap.app.ui.components.SegmentedSettingsColumn
import com.synap.app.ui.components.SegmentedSwitchItem
import com.synap.app.ui.util.navigationBarsScrollPadding
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingColorScreen(
    currentThemeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    onNavigateBack: () -> Unit,
) {
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent -> backProgress = backEvent.progress }
            onNavigateBack()
        } catch (e: CancellationException) {
            backProgress = 0f
        }
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
                title = { Text(stringResource(R.string.setting_color)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp)
                .navigationBarsScrollPadding(),
        ) {
            Spacer(modifier = Modifier.height(16.dp))

            val themeOptions = listOf(
                Icons.Filled.Settings to stringResource(R.string.theme_system),
                Icons.Filled.LightMode to stringResource(R.string.theme_light),
                Icons.Filled.DarkMode to stringResource(R.string.theme_dark),
            )

            SegmentedSettingsColumn(title = stringResource(R.string.dark_mode)) {
                themeOptions.forEachIndexed { index, (icon, title) ->
                    item {
                        SegmentedSelectableItem(
                            title = title,
                            icon = icon,
                            selected = currentThemeMode == index,
                            onClick = { onThemeModeChange(index) },
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            SegmentedSettingsColumn(title = stringResource(R.string.theme_color)) {
                item {
                    SegmentedSwitchItem(
                        icon = Icons.Filled.Palette,
                        title = stringResource(R.string.sync_system_color),
                        summary = when {
                            !supportsMonet -> stringResource(R.string.monet_unsupported)
                            useMonet -> stringResource(R.string.monet_enabled)
                            else -> stringResource(R.string.monet_disabled)
                        },
                        checked = useMonet,
                        enabled = supportsMonet,
                        onCheckedChange = onUseMonetChange,
                    )
                }
                if (!useMonet) {
                    item {
                        SegmentedContentItem {
                            Column(modifier = Modifier.fillMaxWidth()) {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Icon(
                                        imageVector = Icons.Filled.Tune,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.primary,
                                        modifier = Modifier.padding(end = 16.dp)
                                    )
                                    Text(
                                        text = stringResource(R.string.adjust_theme_color),
                                        style = MaterialTheme.typography.bodyLarge,
                                        color = MaterialTheme.colorScheme.onSurface
                                    )
                                }
                                Spacer(modifier = Modifier.height(8.dp))
                                val currentPureColor = Color.hsv(customThemeHue, 1f, 1f)
                                Slider(
                                    value = customThemeHue,
                                    onValueChange = onCustomThemeHueChange,
                                    valueRange = 0f..360f,
                                    colors = SliderDefaults.colors(
                                        thumbColor = currentPureColor,
                                        activeTrackColor = currentPureColor,
                                    )
                                )
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(32.dp))
        }
    }
}
