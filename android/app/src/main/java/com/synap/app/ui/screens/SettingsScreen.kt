package com.synap.app.ui.screens

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Apps
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.FormatSize
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Inventory2
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Link
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Science
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.Restore
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.R
import java.util.concurrent.CancellationException

@Composable
private fun AppearanceSection(
    onNavigateToColorSettings: () -> Unit,
    onNavigateToTypographySettings: () -> Unit,
    onNavigateToLanguageSelection: () -> Unit,
    onNavigateToAppIcon: () -> Unit,
    onNavigateToHomeLayout: () -> Unit,
) {
    Text(
        text = stringResource(R.string.appearance),
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
    )
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.setting_color),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Palette,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToColorSettings() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.language),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Language,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToLanguageSelection() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.setting_app_icon),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Apps,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToAppIcon() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.setting_home_layout),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Home,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToHomeLayout() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.note_typography_style),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.FormatSize,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToTypographySettings() },
        )
    }
}

@Composable
private fun FeatureSection(
    draftCapacity: Int,
    onDraftCapacityChange: (Int) -> Unit,
    onNavigateToLab: () -> Unit,
    onNavigateToShortcut: () -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    val capacities = listOf(0, 5, 10, 20, 50, 100)
    val capacityLabels = listOf(stringResource(R.string.draft_capacity_off), "5", "10", "20", "50", "100")

    Text(
        text = stringResource(R.string.setting_feature),
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
    )
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.CenterEnd) {
            ListItem(
                headlineContent = {
                    Text(
                        text = stringResource(R.string.draft_capacity),
                        style = MaterialTheme.typography.bodyLarge,
                    )
                },
                supportingContent = {
                    Text(
                        text = if (draftCapacity == 0) stringResource(R.string.draft_capacity_summary_off) else stringResource(R.string.draft_capacity_summary_on, draftCapacity),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                },
                leadingContent = {
                    Icon(
                        imageVector = Icons.Filled.Inventory2,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                    )
                },
                trailingContent = {
                    Icon(
                        Icons.Filled.KeyboardArrowRight,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                },
                colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                modifier = Modifier.clickable { expanded = true },
            )

            DropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false },
            ) {
                capacities.forEachIndexed { index, capacity ->
                    DropdownMenuItem(
                        text = {
                            Text(
                                text = capacityLabels[index],
                                style = MaterialTheme.typography.bodyLarge,
                                color = if (draftCapacity == capacity) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                            )
                        },
                        onClick = {
                            onDraftCapacityChange(capacity)
                            expanded = false
                        },
                        trailingIcon = if (draftCapacity == capacity) {
                            {
                                Icon(
                                    imageVector = Icons.Filled.Check,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.primary,
                                )
                            }
                        } else null,
                    )
                }
            }
        }

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        // 快捷记笔记入口
        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.setting_volume_key_shortcut),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Bolt,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToShortcut() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.setting_lab),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Science,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToLab() },
        )
    }
}

@Composable
private fun BackupSyncSection(
    syncStatus: String,
    syncPort: Int?,
    syncAddresses: List<String>,
    onNavigateToSync: () -> Unit,
    onExportDatabase: () -> Unit,
    onShareDatabase: () -> Unit,
    onImportDatabase: () -> Unit,
) {
    Text(
        text = stringResource(R.string.backup_and_sync),
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
    )
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.setting_sync),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Sync,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToSync() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.export_backup),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Save,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onExportDatabase() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.export_and_share),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Share,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onShareDatabase() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(
                    text = stringResource(R.string.import_backup),
                    style = MaterialTheme.typography.bodyLarge,
                )
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Restore,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(
                    Icons.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onImportDatabase() },
        )
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
        modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
    )
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        ListItem(
            headlineContent = {
                Text(stringResource(R.string.app_name), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
            },
            supportingContent = {
                Text(
                    text = stringResource(R.string.app_desc),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    lineHeight = 20.sp,
                )
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(stringResource(R.string.version_info), style = MaterialTheme.typography.bodyLarge)
                    val vType = detectVersionType(buildVersion)
                    if (vType != VersionType.Release) {
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = vType.name,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onPrimaryContainer,
                            modifier = Modifier
                                .background(MaterialTheme.colorScheme.primaryContainer, RoundedCornerShape(4.dp))
                                .padding(horizontal = 6.dp, vertical = 2.dp),
                        )
                    }
                }
            },
            supportingContent = {
                Text(
                    text = buildVersion + (buildVersionDetails?.takeIf { it.isNotBlank() }?.let { " · $it" } ?: ""),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            },
            trailingContent = {
                Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToVersion() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(stringResource(R.string.creative_team), style = MaterialTheme.typography.bodyLarge)
            },
            leadingContent = {
                Icon(
                    imageVector = Icons.Filled.Group,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                )
            },
            trailingContent = {
                Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable { onNavigateToTeam() },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(stringResource(R.string.setting_official_website), style = MaterialTheme.typography.bodyLarge)
            },
            trailingContent = {
                Icon(Icons.Filled.Link, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://vilinkostudio.github.io/synap.vilinko.com/")))
            },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(stringResource(R.string.view_project_homepage), style = MaterialTheme.typography.bodyLarge)
            },
            trailingContent = {
                Icon(Icons.Filled.Link, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/VilinkoStudio/Synap")))
            },
        )

        HorizontalDivider(
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
            modifier = Modifier.padding(horizontal = 16.dp),
        )

        ListItem(
            headlineContent = {
                Text(stringResource(R.string.download_latest_version), style = MaterialTheme.typography.bodyLarge)
            },
            trailingContent = {
                Icon(Icons.Filled.Link, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            modifier = Modifier.clickable {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/VilinkoStudio/Synap/releases")))
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
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
    onNavigateToLab: () -> Unit,
    onNavigateToShortcut: () -> Unit,
    onNavigateBack: () -> Unit,
    draftCapacity: Int,
    onDraftCapacityChange: (Int) -> Unit,
) {
    val context = LocalContext.current

    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress
            }
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
                    onNavigateToLab = onNavigateToLab,
                    onNavigateToShortcut = onNavigateToShortcut,
                )
            }

            val backupSyncContent: @Composable () -> Unit = {
                BackupSyncSection(
                    syncStatus = syncStatus,
                    syncPort = syncPort,
                    syncAddresses = syncAddresses,
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
                            .padding(horizontal = 16.dp),
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
