package com.synap.app.ui.screens

import android.app.KeyguardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.compose.PredictiveBackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Apps
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.filled.FormatSize
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Inventory2
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Link
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Restore
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.components.ExpressiveMenu
import com.synap.app.ui.components.ExpressiveSelectableMenuItem
import com.synap.app.ui.components.SegmentedNavItem
import com.synap.app.ui.components.SegmentedSettingsColumn
import com.synap.app.ui.components.SegmentedSwitchItem
import com.synap.app.ui.util.navigationBarsScrollPadding
import java.util.concurrent.CancellationException

@Composable
private fun AppearanceSection(
    onNavigateToColorSettings: () -> Unit,
    onNavigateToTypographySettings: () -> Unit,
    onNavigateToLanguageSelection: () -> Unit,
    onNavigateToAppIcon: () -> Unit,
    onNavigateToHomeLayout: () -> Unit,
) {
    SegmentedSettingsColumn(title = stringResource(R.string.appearance)) {
        item { SegmentedNavItem(Icons.Filled.Palette, stringResource(R.string.setting_color)) { onNavigateToColorSettings() } }
        item { SegmentedNavItem(Icons.Filled.Language, stringResource(R.string.language)) { onNavigateToLanguageSelection() } }
        item { SegmentedNavItem(Icons.Filled.Apps, stringResource(R.string.setting_app_icon)) { onNavigateToAppIcon() } }
        item { SegmentedNavItem(Icons.Filled.Home, stringResource(R.string.setting_home_layout)) { onNavigateToHomeLayout() } }
        item { SegmentedNavItem(Icons.Filled.FormatSize, stringResource(R.string.note_typography_style)) { onNavigateToTypographySettings() } }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun FeatureSection(
    draftCapacity: Int,
    onDraftCapacityChange: (Int) -> Unit,
    onNavigateToShortcut: () -> Unit,
    securityLockEnabled: Boolean,
    onSecurityLockToggle: (Boolean) -> Unit,
) {
    val context = LocalContext.current
    val biometricLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == android.app.Activity.RESULT_OK) {
            onSecurityLockToggle(true)
        }
    }
    var expanded by remember { mutableStateOf(false) }
    val capacities = listOf(0, 5, 10, 20, 50, 100)
    val capacityLabels = listOf(stringResource(R.string.draft_capacity_off), "5", "10", "20", "50", "100")

    SegmentedSettingsColumn(title = stringResource(R.string.setting_feature)) {
        item {
            Box {
                SegmentedNavItem(
                    icon = Icons.Filled.Inventory2,
                    title = stringResource(R.string.draft_capacity),
                    summary = if (draftCapacity == 0) {
                        stringResource(R.string.draft_capacity_summary_off)
                    } else {
                        stringResource(R.string.draft_capacity_summary_on, draftCapacity)
                    },
                    onClick = { expanded = true },
                )
                ExpressiveMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                    capacities.forEachIndexed { index, capacity ->
                        ExpressiveSelectableMenuItem(
                            text = capacityLabels[index],
                            selected = draftCapacity == capacity,
                            onClick = {
                                onDraftCapacityChange(capacity)
                                expanded = false
                            },
                            index = index,
                            count = capacities.size,
                        )
                    }
                }
            }
        }
        item { SegmentedNavItem(Icons.Filled.Bolt, stringResource(R.string.setting_volume_key_shortcut)) { onNavigateToShortcut() } }
        item {
            SegmentedSwitchItem(
                icon = Icons.Filled.Lock,
                title = stringResource(R.string.setting_security_lock),
                checked = securityLockEnabled,
                onCheckedChange = { wantEnabled ->
                    if (!wantEnabled) {
                        onSecurityLockToggle(false)
                    } else {
                        val km = context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
                        val intent = km.createConfirmDeviceCredentialIntent(
                            context.getString(R.string.setting_security_lock),
                            context.getString(R.string.setting_security_lock),
                        )
                        if (intent != null) biometricLauncher.launch(intent) else onSecurityLockToggle(true)
                    }
                },
            )
        }
    }
}

@Composable
private fun BackupSyncSection(
    onNavigateToSync: () -> Unit,
    onExportDatabase: () -> Unit,
    onShareDatabase: () -> Unit,
    onImportDatabase: () -> Unit,
) {
    SegmentedSettingsColumn(title = stringResource(R.string.backup_and_sync)) {
        item { SegmentedNavItem(Icons.Filled.Sync, stringResource(R.string.setting_sync)) { onNavigateToSync() } }
        item { SegmentedNavItem(Icons.Filled.Save, stringResource(R.string.export_backup)) { onExportDatabase() } }
        item { SegmentedNavItem(Icons.Filled.Share, stringResource(R.string.export_and_share)) { onShareDatabase() } }
        item { SegmentedNavItem(Icons.Filled.Restore, stringResource(R.string.import_backup)) { onImportDatabase() } }
    }
}

@Composable
private fun AboutSection(
    buildVersion: String,
    buildVersionDetails: String?,
    onNavigateToTeam: () -> Unit,
    onNavigateToVersion: () -> Unit,
) {
    val context = LocalContext.current

    Text(
        text = stringResource(R.string.about),
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 8.dp),
    )
    Spacer(modifier = Modifier.height(12.dp))
    Column(modifier = Modifier.padding(horizontal = 8.dp)) {
        Text(
            text = stringResource(R.string.app_name),
            style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.Bold),
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            text = stringResource(R.string.app_desc),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
    Spacer(modifier = Modifier.height(12.dp))

    SegmentedSettingsColumn {
        item {
            val versionType = detectVersionType(buildVersion)
            SegmentedNavItem(
                icon = null,
                title = stringResource(R.string.version_info),
                summary = buildVersion + (buildVersionDetails?.takeIf { it.isNotBlank() }?.let { " · $it" } ?: ""),
                badge = versionType.takeIf { it != VersionType.Release }?.name,
                onClick = onNavigateToVersion,
            )
        }
        item { SegmentedNavItem(Icons.Filled.Group, stringResource(R.string.creative_team)) { onNavigateToTeam() } }
        item {
            SegmentedNavItem(Icons.Filled.Link, stringResource(R.string.setting_official_website)) {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://synap.rth1.xyz/")))
            }
        }
        item {
            SegmentedNavItem(Icons.Filled.Link, stringResource(R.string.view_project_homepage)) {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/VilinkoStudio/Synap")))
            }
        }
        item {
            SegmentedNavItem(Icons.Filled.Link, stringResource(R.string.download_latest_version)) {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/VilinkoStudio/Synap/releases")))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SettingsScreen(
    currentThemeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    handedness: String,
    onHandednessChange: (String) -> Unit,
    buildVersion: String,
    buildVersionDetails: String?,
    syncStatus: String,
    syncPort: Int?,
    syncAddresses: List<String>,
    onExportNotes: () -> Unit,
    onExportDatabase: () -> Unit,
    onShareDatabase: () -> Unit,
    onImportDatabase: () -> Unit,
    onNavigateToTypographySettings: () -> Unit,
    onNavigateToColorSettings: () -> Unit,
    onNavigateToLanguageSelection: () -> Unit,
    onNavigateToAppIcon: () -> Unit,
    onNavigateToHomeLayout: () -> Unit,
    onNavigateToSync: () -> Unit,
    onNavigateToTeam: () -> Unit,
    onNavigateToVersion: () -> Unit,
    onNavigateToShortcut: () -> Unit,
    onNavigateBack: () -> Unit,
    draftCapacity: Int,
    onDraftCapacityChange: (Int) -> Unit,
    securityLockEnabled: Boolean,
    onSecurityLockToggle: (Boolean) -> Unit,
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
                title = { Text(stringResource(R.string.settings)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
            )
        },
    ) { innerPadding ->
        BoxWithConstraints(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            val isWideScreen = maxWidth >= 1000.dp
            val isMediumScreen = maxWidth >= 700.dp

            val appearanceContent: @Composable () -> Unit = {
                AppearanceSection(
                    onNavigateToColorSettings = onNavigateToColorSettings,
                    onNavigateToTypographySettings = onNavigateToTypographySettings,
                    onNavigateToLanguageSelection = onNavigateToLanguageSelection,
                    onNavigateToAppIcon = onNavigateToAppIcon,
                    onNavigateToHomeLayout = onNavigateToHomeLayout,
                )
            }

            val functionContent: @Composable () -> Unit = {
                FeatureSection(
                    draftCapacity = draftCapacity,
                    onDraftCapacityChange = onDraftCapacityChange,
                    onNavigateToShortcut = onNavigateToShortcut,
                    securityLockEnabled = securityLockEnabled,
                    onSecurityLockToggle = onSecurityLockToggle,
                )
            }

            val backupSyncContent: @Composable () -> Unit = {
                BackupSyncSection(
                    onNavigateToSync = onNavigateToSync,
                    onExportDatabase = onExportDatabase,
                    onShareDatabase = onShareDatabase,
                    onImportDatabase = onImportDatabase,
                )
            }

            val aboutContent: @Composable () -> Unit = {
                AboutSection(
                    buildVersion = buildVersion,
                    buildVersionDetails = buildVersionDetails,
                    onNavigateToTeam = onNavigateToTeam,
                    onNavigateToVersion = onNavigateToVersion,
                )
            }

            when {
                isWideScreen -> {
                    Row(
                        modifier = Modifier
                            .fillMaxSize()
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 16.dp)
                            .navigationBarsScrollPadding(24.dp)
                    ) {
                        Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                            appearanceContent()
                        }
                        Column(modifier = Modifier.weight(1f).padding(horizontal = 8.dp)) {
                            functionContent()
                            Spacer(modifier = Modifier.height(24.dp))
                            backupSyncContent()
                        }
                        Column(modifier = Modifier.weight(1f).padding(start = 8.dp)) {
                            aboutContent()
                        }
                    }
                }
                isMediumScreen -> {
                    Row(
                        modifier = Modifier
                            .fillMaxSize()
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 16.dp)
                            .navigationBarsScrollPadding(24.dp)
                    ) {
                        Column(modifier = Modifier.weight(1f).padding(end = 8.dp)) {
                            appearanceContent()
                            Spacer(modifier = Modifier.height(24.dp))
                            backupSyncContent()
                        }
                        Column(modifier = Modifier.weight(1f).padding(start = 8.dp)) {
                            functionContent()
                            Spacer(modifier = Modifier.height(24.dp))
                            aboutContent()
                        }
                    }
                }
                else -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 16.dp)
                            .navigationBarsScrollPadding(24.dp),
                    ) {
                        appearanceContent()
                        Spacer(modifier = Modifier.height(24.dp))
                        functionContent()
                        Spacer(modifier = Modifier.height(24.dp))
                        backupSyncContent()
                        Spacer(modifier = Modifier.height(24.dp))
                        aboutContent()
                        Spacer(modifier = Modifier.height(32.dp))
                    }
                }
            }
        }
    }
}
