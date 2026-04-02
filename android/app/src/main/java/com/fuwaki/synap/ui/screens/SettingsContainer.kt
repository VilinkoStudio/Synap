package com.fuwaki.synap.ui.screens

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
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.FileProvider
import androidx.hilt.navigation.compose.hiltViewModel
import com.fuwaki.synap.MainActivity
import com.fuwaki.synap.ui.viewmodel.SettingsViewModel
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
    onNavigateToLanguageSelection: () -> Unit,
    onNavigateToTutorial: () -> Unit, // --- 加上这个缺失的参数 ---
    onNavigateBack: () -> Unit
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
                .onSuccess { Toast.makeText(context, "数据库已导出", Toast.LENGTH_SHORT).show() }
                .onFailure { Toast.makeText(context, it.message ?: "导出失败", Toast.LENGTH_LONG).show() }
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
                    .onFailure { Toast.makeText(context, it.message ?: "导入失败", Toast.LENGTH_LONG).show() }
            }
        }
    }

    if (showImportWarning) {
        AlertDialog(
            onDismissRequest = { showImportWarning = false },
            title = { Text("导入备份并替换") },
            text = { Text("注意！导入备份文件后，原来的数据将会被清空，请谨慎操作。导入后需要重启 App 生效") },
            confirmButton = {
                TextButton(onClick = { showImportWarning = false; importDatabaseLauncher.launch(arrayOf("*/*")) }) { Text("选择数据库") }
            },
            dismissButton = {
                TextButton(onClick = { showImportWarning = false }) { Text("取消") }
            }
        )
    }

    if (showFileTypeError) {
        AlertDialog(
            onDismissRequest = { showFileTypeError = false },
            title = { Text("文件类型错误") },
            text = { Text("请上传“.redb”格式的备份文件") },
            confirmButton = {
                TextButton(onClick = { showFileTypeError = false; importDatabaseLauncher.launch(arrayOf("*/*")) }) { Text("重新选择") }
            },
            dismissButton = {
                TextButton(onClick = { showFileTypeError = false }) { Text("取消") }
            }
        )
    }

    if (showRestartRequired) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text("需要重启应用") },
            text = { Text("已成功导入备份数据，重启应用后生效。") },
            confirmButton = {
                TextButton(onClick = { showRestartRequired = false; databaseActivity?.closeForDatabaseRestart() }) { Text("关闭应用") }
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
        onExportNotes = {
            scope.launch(Dispatchers.IO) {
                val testJsonData = "[\n  {\n    \"id\": \"1\",\n    \"content\": \"这是一条导出测试笔记。\"\n  }\n]"
                withContext(Dispatchers.Main) { exportDataToZipAndShare(context, testJsonData) }
            }
        },
        onExportDatabase = { exportDatabaseLauncher.launch("synap_database.redb") },
        onShareDatabase = {
            if (databaseActivity == null) {
                Toast.makeText(context, "当前无法分享数据库", Toast.LENGTH_SHORT).show()
            } else {
                scope.launch {
                    databaseActivity.shareDatabase()
                        .onFailure { Toast.makeText(context, it.message ?: "分享失败", Toast.LENGTH_LONG).show() }
                }
            }
        },
        onImportDatabase = {
            if (databaseActivity == null) {
                Toast.makeText(context, "当前无法导入数据库", Toast.LENGTH_SHORT).show()
            } else {
                showImportWarning = true
            }
        },
        onNavigateToTypographySettings = onNavigateToTypographySettings,
        onNavigateToLanguageSelection = onNavigateToLanguageSelection,
        onNavigateToTutorial = onNavigateToTutorial, // --- 向下传递给 SettingsScreen ---
        onNavigateBack = onNavigateBack,
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
        context.startActivity(Intent.createChooser(shareIntent, "导出备份文件"))
    } catch (e: Exception) {
        e.printStackTrace()
    }
}