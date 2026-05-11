package com.synap.app

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import com.synap.app.data.service.SynapServiceApi
import com.synap.app.ui.viewmodel.ShareImportViewModel
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : AppCompatActivity() {
    @Inject
    lateinit var synapService: SynapServiceApi

    private val shareImportViewModel: ShareImportViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // ========== 处理冷启动时的外部文字传入 ==========
        handleExternalTextIntent(intent)

        setContent {
            SynapApp(activity = this)
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)

        // ========== 核心修复：处理热启动（App在后台） ==========
        // 如果成功拦截到分享文字，强制通过 CLEAR_TASK 重启，以完美触发 Compose 的深层链接
        if (handleExternalTextIntent(intent)) {
            val restartIntent = Intent(intent).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            }
            startActivity(restartIntent)
        }
    }

    // ========== 逻辑升级：返回 Boolean，并清理已处理的数据 ==========
    private fun handleExternalTextIntent(intent: Intent?): Boolean {
        if (intent == null) return false

        // 如果是导入分享记录等意图，处理后直接返回 false，不需要为了它而重启页面
        if (handleImportShareIntent(intent)) {
            return false // <--- 修复了这里的编译报错
        }

        var extractedText: String? = null

        // 1. 处理系统选词菜单
        if (intent.action == Intent.ACTION_PROCESS_TEXT) {
            extractedText = intent.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)?.toString()
        }
        // 2. 处理系统分享菜单 (放宽了类型判断，兼容更多 App 传来的文本)
        else if (intent.action == Intent.ACTION_SEND && intent.type?.startsWith("text/") == true) {
            extractedText = intent.getStringExtra(Intent.EXTRA_TEXT)
        }

        // 如果成功提取到文字，统一转换为新建笔记的 DeepLink
        if (!extractedText.isNullOrBlank()) {
            // 将 Intent 篡改为单纯的 DeepLink
            intent.action = Intent.ACTION_VIEW
            intent.data = Uri.parse("synap://editor?initialContent=${Uri.encode(extractedText)}")

            // 重要：清理掉原有的文本 Extra，防止页面重启后被重复处理
            intent.removeExtra(Intent.EXTRA_PROCESS_TEXT)
            intent.removeExtra(Intent.EXTRA_TEXT)
            return true
        }
        return false
    }

    private fun handleImportShareIntent(intent: Intent): Boolean {
        val data = intent.data ?: return false
        if (intent.action != Intent.ACTION_VIEW || data.host != "import_share") {
            return false
        }

        shareImportViewModel.importFromDeepLink(data)
        intent.data = null
        return true
    }

    suspend fun exportDatabaseToUri(uri: Uri): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            contentResolver.openOutputStream(uri)?.use { output ->
                synapService.exportDatabase(output).getOrThrow()
            } ?: error("无法创建导出文件")
        }
    }

    suspend fun shareDatabase(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val cachePath = File(cacheDir, "exports").apply { mkdirs() }
            val exportFile = File(cachePath, "synap_database.redb")
            exportFile.outputStream().use { output ->
                synapService.exportDatabase(output).getOrThrow()
            }
            val authority = "$packageName.fileprovider"
            val uri = FileProvider.getUriForFile(this@MainActivity, authority, exportFile)
            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = "application/octet-stream"
                putExtra(Intent.EXTRA_STREAM, uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            withContext(Dispatchers.Main) {
                startActivity(Intent.createChooser(shareIntent, "分享数据库"))
            }
        }
    }

    suspend fun importDatabaseFromUri(uri: Uri): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            contentResolver.openInputStream(uri)?.use { input ->
                synapService.replaceDatabase(input).getOrThrow()
            } ?: error("无法读取导入文件")
        }
    }

    fun closeForDatabaseRestart() {
        finishAffinity()
    }
}