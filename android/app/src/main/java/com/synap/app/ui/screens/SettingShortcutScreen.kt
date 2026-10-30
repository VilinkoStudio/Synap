package com.synap.app.ui.screens

import android.app.StatusBarManager
import android.content.Intent
import android.graphics.drawable.Icon
import android.os.Build
import android.provider.Settings
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
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import com.synap.app.ShizukuHelper
import com.synap.app.ui.components.SegmentedContentItem
import com.synap.app.ui.components.SegmentedNavItem
import com.synap.app.ui.components.SegmentedSettingsColumn
import com.synap.app.ui.util.navigationBarsScrollPadding
import com.synap.app.widget.QuickNoteTileService
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingShortcutScreen(
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    var backProgress by remember { mutableFloatStateOf(0f) }
    var shizukuAuthorized by remember { mutableStateOf(ShizukuHelper.isShizukuPermissionGranted()) }
    var shizukuRunning by remember { mutableStateOf(ShizukuHelper.isShizukuRunning()) }

    LaunchedEffect(Unit) {
        shizukuRunning = ShizukuHelper.isShizukuRunning()
        shizukuAuthorized = ShizukuHelper.isShizukuPermissionGranted()
    }

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
                title = { Text(stringResource(R.string.setting_shortcut_title)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
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
            SegmentedSettingsColumn(title = stringResource(R.string.shortcut_method1_title)) {
                item {
                    SegmentedNavItem(
                        icon = null,
                        title = stringResource(R.string.shortcut_shizuku_grant),
                        summary = when {
                            !shizukuRunning -> stringResource(R.string.shizuku_status_not_running)
                            shizukuAuthorized -> stringResource(R.string.shizuku_status_authorized)
                            else -> stringResource(R.string.shizuku_status_unauthorized)
                        },
                        onClick = {
                            ShizukuHelper.checkPermission { granted ->
                                shizukuAuthorized = granted
                                if (granted) {
                                    ShizukuHelper.grantWriteSecureSettings(context) { writeGranted ->
                                        if (writeGranted) {
                                            ShizukuHelper.setAccessibilityShortcut(context) { shortcutGranted ->
                                                android.os.Handler(android.os.Looper.getMainLooper()).post {
                                                    Toast.makeText(
                                                        context,
                                                        if (shortcutGranted) R.string.shizuku_setup_success else R.string.shizuku_setup_failed,
                                                        Toast.LENGTH_SHORT,
                                                    ).show()
                                                    shizukuAuthorized = ShizukuHelper.isShizukuPermissionGranted()
                                                }
                                            }
                                        } else {
                                            android.os.Handler(android.os.Looper.getMainLooper()).post {
                                                Toast.makeText(context, R.string.shizuku_grant_failed, Toast.LENGTH_SHORT).show()
                                            }
                                        }
                                    }
                                } else {
                                    android.os.Handler(android.os.Looper.getMainLooper()).post {
                                        Toast.makeText(context, R.string.shizuku_permission_denied, Toast.LENGTH_SHORT).show()
                                    }
                                }
                            }
                        },
                    )
                }
                item {
                    SegmentedNavItem(
                        icon = null,
                        title = stringResource(R.string.shortcut_accessibility_grant),
                        summary = stringResource(R.string.shortcut_accessibility_grant_desc),
                        onClick = {
                            context.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).apply {
                                flags = Intent.FLAG_ACTIVITY_NEW_TASK
                            })
                        },
                    )
                }
            }

            Text(
                text = stringResource(R.string.shortcut_accessibility_notice),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 8.dp, vertical = 12.dp),
            )

            SegmentedSettingsColumn(title = stringResource(R.string.shortcut_method2_title)) {
                item {
                    SegmentedNavItem(
                        icon = null,
                        title = stringResource(R.string.shortcut_add_tile),
                        onClick = {
                            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                                val statusBarManager = context.getSystemService(StatusBarManager::class.java)
                                val componentName = android.content.ComponentName(context, QuickNoteTileService::class.java)
                                val icon = Icon.createWithResource(context, android.R.drawable.ic_menu_edit)
                                val executor = java.util.concurrent.Executors.newSingleThreadExecutor()
                                statusBarManager?.requestAddTileService(
                                    componentName,
                                    context.getString(R.string.app_name),
                                    icon,
                                    executor,
                                    java.util.function.Consumer { }
                                )
                            } else {
                                Toast.makeText(context, R.string.shortcut_add_tile_hint, Toast.LENGTH_LONG).show()
                            }
                        },
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            SegmentedSettingsColumn(title = stringResource(R.string.shortcut_method3_title)) {
                item {
                    SegmentedContentItem {
                        Text(
                            text = stringResource(R.string.shortcut_method3_widget_guide),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            SegmentedSettingsColumn(title = stringResource(R.string.shortcut_method4_title)) {
                item {
                    SegmentedContentItem {
                        Column {
                            Text(
                                text = stringResource(R.string.shortcut_method4_desc),
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                            SelectionContainer {
                                Text(
                                    text = stringResource(R.string.shortcut_method4_sample),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurface,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
