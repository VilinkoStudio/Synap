package com.synap.app.ui.screens

import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.widget.Toast
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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.SegmentedSelectableItem
import com.synap.app.ui.components.SegmentedSettingsColumn
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingLogoScreen(onNavigateBack: () -> Unit) {
    val context = LocalContext.current
    val packageManager = context.packageManager
    val packageName = context.packageName

    val oldComponent = ComponentName(context, "$packageName.MainActivityOld")
    val defaultComponent = ComponentName(context, "$packageName.MainActivity")

    val resolveIntent = Intent(Intent.ACTION_MAIN).apply {
        addCategory(Intent.CATEGORY_LAUNCHER)
        setPackage(packageName)
    }
    val resolvedActivity = packageManager.resolveActivity(resolveIntent, PackageManager.MATCH_DEFAULT_ONLY)?.activityInfo?.name
    val isOldIconCurrentlyEnabled = resolvedActivity == "$packageName.MainActivityOld"

    var selectedIndex by remember { mutableIntStateOf(if (isOldIconCurrentlyEnabled) 1 else 0) }
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent -> backProgress = backEvent.progress }
            onNavigateBack()
        } catch (e: CancellationException) {
            backProgress = 0f
        }
    }

    fun switchIcon(index: Int) {
        if (selectedIndex == index) return
        selectedIndex = index

        if (index == 1) {
            packageManager.setComponentEnabledSetting(oldComponent, PackageManager.COMPONENT_ENABLED_STATE_ENABLED, PackageManager.DONT_KILL_APP)
            packageManager.setComponentEnabledSetting(defaultComponent, PackageManager.COMPONENT_ENABLED_STATE_DISABLED, PackageManager.DONT_KILL_APP)
        } else {
            packageManager.setComponentEnabledSetting(defaultComponent, PackageManager.COMPONENT_ENABLED_STATE_ENABLED, PackageManager.DONT_KILL_APP)
            packageManager.setComponentEnabledSetting(oldComponent, PackageManager.COMPONENT_ENABLED_STATE_DISABLED, PackageManager.DONT_KILL_APP)
        }
        Toast.makeText(context, context.getString(R.string.icon_switched_toast), Toast.LENGTH_SHORT).show()
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
                title = { Text(stringResource(R.string.setting_app_icon)) },
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
                .padding(16.dp)
        ) {
            val options = listOf(stringResource(R.string.icon_synap_colored), stringResource(R.string.icon_monet))

            SegmentedSettingsColumn(title = stringResource(R.string.desktop_icon_style)) {
                options.forEachIndexed { index, title ->
                    item {
                        SegmentedSelectableItem(
                            title = title,
                            selected = selectedIndex == index,
                            onClick = { switchIcon(index) },
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Text(
                text = stringResource(R.string.icon_switch_warning),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 8.dp)
            )
        }
    }
}
