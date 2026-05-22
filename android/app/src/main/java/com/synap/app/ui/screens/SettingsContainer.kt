package com.synap.app.ui.screens

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.FileProvider
import com.synap.app.R
import androidx.hilt.navigation.compose.hiltViewModel
import com.synap.app.MainActivity
import com.synap.app.ui.viewmodel.SettingsViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

@Composable
fun SettingsContainer(
    themeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    handedness: String,
    onHandednessChange: (String) -> Unit,
    databaseActivity: MainActivity?,
    onNavigateToTypographySettings: () -> Unit,
    onNavigateToColorSettings: () -> Unit,
    onNavigateToLanguageSelection: () -> Unit,
    onNavigateToAppIcon: () -> Unit,
    onNavigateToHomeLayout: () -> Unit,
    onNavigateToLab: () -> Unit,
    onNavigateToAIService: () -> Unit,
    onNavigateToAIScenarios: () -> Unit,
    onNavigateToSync: () -> Unit,
    onNavigateToTeam: () -> Unit,
    onNavigateToTutorial: () -> Unit,
    onNavigateBack: () -> Unit,
    draftCapacity: Int,
    onDraftCapacityChange: (Int) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val settingsViewModel: SettingsViewModel = hiltViewModel()
    val settingsUiState by settingsViewModel.uiState.collectAsState()

    var showImportWarning by remember { mutableStateOf(false) }
    var showRestartRequired by remember { mutableStateOf(false) }
    var showFileTypeError by remember { mutableStateOf(false) }

    val exportDatabaseLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("application/octet-stream"),
    ) { uri ->
        if (uri == null || databaseActivity == null) return@rememberLauncherForActivityResult
        scope.launch {
            databaseActivity.exportDatabaseToUri(uri)
                .onSuccess { Toast.makeText(context, context.getString(R.string.database_exported), Toast.LENGTH_SHORT).show() }
                .onFailure { Toast.makeText(context, it.message ?: context.getString(R.string.export_failed), Toast.LENGTH_LONG).show() }
        }
    }

    val importDatabaseLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null || databaseActivity == null) return@rememberLauncherForActivityResult

        var isRedbFile = false
        context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) {
                val displayNameIndex = cursor.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                if (displayNameIndex != -1) {
                    val displayName = cursor.getString(displayNameIndex)
                    if (displayName != null && displayName.endsWith(".redb", ignoreCase = true)) {
                        isRedbFile = true
                    }
                }
            }
        }

        if (!isRedbFile) {
            showFileTypeError = true
        } else {
            scope.launch {
                databaseActivity.importDatabaseFromUri(uri)
                    .onSuccess { showRestartRequired = true }
                    .onFailure { Toast.makeText(context, it.message ?: context.getString(R.string.import_failed), Toast.LENGTH_LONG).show() }
            }
        }
    }

    if (showImportWarning) {
        AlertDialog(
            onDismissRequest = { showImportWarning = false },
            title = { Text(stringResource(R.string.import_backup_title)) },
            text = { Text(stringResource(R.string.import_backup_warning)) },
            confirmButton = {
                TextButton(onClick = { showImportWarning = false; importDatabaseLauncher.launch(arrayOf("*/*")) }) { Text(stringResource(R.string.select_database)) }
            },
            dismissButton = {
                TextButton(onClick = { showImportWarning = false }) { Text(stringResource(R.string.cancel)) }
            }
        )
    }

    if (showFileTypeError) {
        AlertDialog(
            onDismissRequest = { showFileTypeError = false },
            title = { Text(stringResource(R.string.file_type_error)) },
            text = { Text(stringResource(R.string.file_type_error_message)) },
            confirmButton = {
                TextButton(onClick = { showFileTypeError = false; importDatabaseLauncher.launch(arrayOf("*/*")) }) { Text(stringResource(R.string.reselect)) }
            },
            dismissButton = {
                TextButton(onClick = { showFileTypeError = false }) { Text(stringResource(R.string.cancel)) }
            }
        )
    }

    if (showRestartRequired) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text(stringResource(R.string.restart_required)) },
            text = { Text(stringResource(R.string.import_success_restart)) },
            confirmButton = {
                TextButton(onClick = { showRestartRequired = false; databaseActivity?.closeForDatabaseRestart() }) { Text(stringResource(R.string.close_app)) }
            }
        )
    }

    SettingsScreen(
        currentThemeMode = themeMode,
        onThemeModeChange = onThemeModeChange,
        useMonet = useMonet,
        supportsMonet = supportsMonet,
        onUseMonetChange = onUseMonetChange,
        customThemeHue = customThemeHue,
        onCustomThemeHueChange = onCustomThemeHueChange,
        handedness = handedness,
        onHandednessChange = onHandednessChange,
        buildVersion = settingsUiState.buildVersion,
        buildVersionDetails = settingsUiState.buildVersionDetails,
        syncStatus = settingsUiState.syncStatus,
        syncPort = settingsUiState.syncPort,
        syncAddresses = settingsUiState.syncAddresses,
        onExportNotes = {
            scope.launch(Dispatchers.IO) {
                val testJsonData = "[\n  {\n    \"id\": \"1\",\n    \"content\": \"这是一条导出测试笔记。\"\n  }\n]"
                withContext(Dispatchers.Main) { exportDataToZipAndShare(context, testJsonData) }
            }
        },
        onExportDatabase = { exportDatabaseLauncher.launch("synap_database.redb") },
        onShareDatabase = {
            if (databaseActivity == null) {
                Toast.makeText(context, context.getString(R.string.cannot_share_database), Toast.LENGTH_SHORT).show()
            } else {
                scope.launch {
                    databaseActivity.shareDatabase()
                        .onFailure { Toast.makeText(context, it.message ?: context.getString(R.string.share_failed), Toast.LENGTH_LONG).show() }
                }
            }
        },
        onImportDatabase = {
            if (databaseActivity == null) {
                Toast.makeText(context, context.getString(R.string.cannot_import_database), Toast.LENGTH_SHORT).show()
            } else {
                showImportWarning = true
            }
        },
        onNavigateToTypographySettings = onNavigateToTypographySettings,
        onNavigateToColorSettings = onNavigateToColorSettings,
        onNavigateToLanguageSelection = onNavigateToLanguageSelection,
        onNavigateToAppIcon = onNavigateToAppIcon,
        onNavigateToHomeLayout = onNavigateToHomeLayout,
        onNavigateToLab = onNavigateToLab,
        onNavigateToAIService = onNavigateToAIService,
        onNavigateToAIScenarios = onNavigateToAIScenarios,
        onNavigateToSync = onNavigateToSync,
        onNavigateToTeam = onNavigateToTeam,
        onNavigateToTutorial = onNavigateToTutorial,
        onNavigateBack = onNavigateBack,
        draftCapacity = draftCapacity,
        onDraftCapacityChange = onDraftCapacityChange,
    )
}

fun exportDataToZipAndShare(context: Context, jsonData: String) {
    try {
        val cachePath = File(context.cacheDir, "exports").apply { mkdirs() }
        val zipFile = File(cachePath, "synap_backup.zip")

        ZipOutputStream(FileOutputStream(zipFile)).use { zos ->
            val entry = ZipEntry("synap_notes.json")
            zos.putNextEntry(entry)
            zos.write(jsonData.toByteArray(Charsets.UTF_8))
            zos.closeEntry()
        }

        val authority = "${context.packageName}.fileprovider"
        val uri: Uri = FileProvider.getUriForFile(context, authority, zipFile)

        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "application/zip"
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(shareIntent, context.getString(R.string.export_backup_title)))
    } catch (e: Exception) {
        e.printStackTrace()
    }
}
