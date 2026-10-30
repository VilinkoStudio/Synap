package com.synap.app.ui.screens

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.synap.app.R
import com.synap.app.ui.util.navigationBarsScrollPadding
import com.synap.app.ui.components.SegmentedInfoItem
import com.synap.app.ui.components.SegmentedNavItem
import com.synap.app.ui.components.SegmentedSelectableItem
import com.synap.app.ui.components.SegmentedSettingsColumn
import com.synap.app.ui.components.SegmentedSwitchItem
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONArray
import java.net.HttpURLConnection
import java.net.URL

enum class VersionType { Release, Beta, Alpha, Dev }

data class GithubRelease(
    val tagName: String,
    val versionName: String,
    val body: String,
    val htmlUrl: String,
    val isPrerelease: Boolean,
)

fun detectVersionType(displayVersion: String): VersionType {
    val lower = displayVersion.lowercase()
    return when {
        "alpha" in lower -> VersionType.Alpha
        "beta" in lower -> VersionType.Beta
        "release" in lower -> VersionType.Release
        else -> VersionType.Dev
    }
}

fun parseVersionNumber(tagName: String): List<Int> {
    val cleaned = tagName.removePrefix("android-")
        .replace(Regex("-?(alpha|beta|release).*"), "")
    return cleaned.split(".").mapNotNull { it.toIntOrNull() }
}

fun isVersionHigher(newTag: String, currentTag: String): Boolean {
    val newParts = parseVersionNumber(newTag)
    val currentParts = parseVersionNumber(currentTag)
    for (i in 0 until maxOf(newParts.size, currentParts.size)) {
        val n = newParts.getOrElse(i) { 0 }
        val c = currentParts.getOrElse(i) { 0 }
        if (n > c) return true
        if (n < c) return false
    }
    return false
}

suspend fun fetchGithubReleases(): List<GithubRelease> = withContext(Dispatchers.IO) {
    val url = URL("https://api.github.com/repos/VilinkoStudio/Synap/releases?per_page=20")
    val conn = url.openConnection() as HttpURLConnection
    conn.requestMethod = "GET"
    conn.setRequestProperty("Accept", "application/vnd.github.v3+json")
    conn.connectTimeout = 5000
    conn.readTimeout = 5000
    try {
        val text = conn.inputStream.bufferedReader().readText()
        val arr = JSONArray(text)
        (0 until arr.length()).mapNotNull { i ->
            val obj = arr.getJSONObject(i)
            val tag = obj.getString("tag_name")
            if (!tag.startsWith("android-")) return@mapNotNull null
            GithubRelease(
                tagName = tag,
                versionName = obj.optString("name", tag),
                body = obj.optString("body", ""),
                htmlUrl = obj.getString("html_url"),
                isPrerelease = obj.getBoolean("prerelease"),
            )
        }
    } finally {
        conn.disconnect()
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VersionScreen(
    buildVersion: String,
    buildVersionDetails: String?,
    onNavigateBack: () -> Unit,
) {
    var backProgress by remember { mutableFloatStateOf(0f) }
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("synap_prefs", Context.MODE_PRIVATE) }
    val scope = rememberCoroutineScope()

    val versionType = remember(buildVersion) { detectVersionType(buildVersion) }

    var autoCheck by remember { mutableStateOf(prefs.getBoolean("auto_check_update", false)) }
    var acceptRelease by remember { mutableStateOf(prefs.getBoolean("accept_release", true)) }
    var acceptBeta by remember { mutableStateOf(prefs.getBoolean("accept_beta", false)) }
    var acceptAlpha by remember { mutableStateOf(prefs.getBoolean("accept_alpha", false)) }
    var isChannelExpanded by remember { mutableStateOf(false) }
    var isChecking by remember { mutableStateOf(false) }
    var updateDialogData by remember { mutableStateOf<GithubRelease?>(null) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress
            }
            onNavigateBack()
        } catch (_: CancellationException) {
            backProgress = 0f
        }
    }

    LaunchedEffect(Unit) {
        if (autoCheck && versionType != VersionType.Dev) {
            isChecking = true
            val release = withTimeoutOrNull(10000L) {
                try {
                    val releases = fetchGithubReleases()
                    val currentTag = buildVersion.substringBefore(" ").trim()
                    val matching = releases.filter { rel ->
                        val isHigher = isVersionHigher(rel.tagName, currentTag)
                        val typeMatch = when {
                            rel.tagName.contains("alpha") -> acceptAlpha
                            rel.tagName.contains("beta") -> acceptBeta
                            else -> acceptRelease
                        }
                        isHigher && typeMatch
                    }
                    matching.maxByOrNull { it.tagName }
                } catch (_: Exception) { null }
            }
            if (release != null) {
                updateDialogData = release
            }
            isChecking = false
        }
    }

    if (updateDialogData != null) {
        val release = updateDialogData!!
        AlertDialog(
            onDismissRequest = { updateDialogData = null },
            title = { Text("检测到新版本更新") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        text = release.versionName,
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    if (release.body.isNotBlank()) {
                        Text(
                            text = release.body,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            },
            confirmButton = {
                Button(onClick = {
                    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(release.htmlUrl)))
                    updateDialogData = null
                }) { Text("更新") }
            },
            dismissButton = {
                TextButton(onClick = { updateDialogData = null }) { Text("取消") }
            }
        )
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
                title = { Text(stringResource(R.string.version_info)) },
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
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Spacer(modifier = Modifier.height(4.dp))

            // 版本信息
            SegmentedSettingsColumn(title = "版本信息") {
                item {
                    SegmentedInfoItem(
                        icon = null,
                        title = "当前版本",
                        summary = buildVersion.substringBefore("·").trim(),
                    )
                }
                item {
                    val typeLabel = when (versionType) {
                        VersionType.Release -> "正式版"
                        VersionType.Beta -> "Beta 测试版"
                        VersionType.Alpha -> "Alpha 测试版"
                        VersionType.Dev -> "Developer 开发版本"
                    }
                    SegmentedInfoItem(icon = null, title = "类型", summary = typeLabel)
                }
            }

            // 更新选项
            SegmentedSettingsColumn(title = "更新选项") {
                item {
                    SegmentedSwitchItem(
                        icon = null,
                        title = "自动检测新版本",
                        summary = if (versionType == VersionType.Dev) {
                            "Debug版本无法进行自动检测，请切换到已构建版本后重试"
                        } else {
                            null
                        },
                        checked = autoCheck,
                        enabled = versionType != VersionType.Dev,
                        onCheckedChange = {
                            autoCheck = it
                            prefs.edit().putBoolean("auto_check_update", it).apply()
                        },
                    )
                }
                item {
                    val selectedChannels = buildList {
                        if (acceptRelease) add("Release")
                        if (acceptBeta) add("Beta")
                        if (acceptAlpha) add("Alpha")
                    }.joinToString("，")
                    SegmentedNavItem(
                        icon = null,
                        title = "接收哪些版本的更新",
                        summary = selectedChannels.ifEmpty { "未选择" },
                        onClick = { isChannelExpanded = !isChannelExpanded },
                    )
                }
                if (isChannelExpanded) {
                    item {
                        SegmentedSelectableItem(
                            title = "Release",
                            selected = acceptRelease,
                            indent = true,
                            onClick = {
                                if (!acceptRelease || acceptBeta || acceptAlpha) {
                                    acceptRelease = !acceptRelease
                                    prefs.edit().putBoolean("accept_release", acceptRelease).apply()
                                }
                            },
                        )
                    }
                    item {
                        SegmentedSelectableItem(
                            title = "Beta",
                            selected = acceptBeta,
                            indent = true,
                            onClick = {
                                if (!acceptBeta || acceptRelease || acceptAlpha) {
                                    acceptBeta = !acceptBeta
                                    prefs.edit().putBoolean("accept_beta", acceptBeta).apply()
                                }
                            },
                        )
                    }
                    item {
                        SegmentedSelectableItem(
                            title = "Alpha",
                            selected = acceptAlpha,
                            indent = true,
                            onClick = {
                                if (!acceptAlpha || acceptRelease || acceptBeta) {
                                    acceptAlpha = !acceptAlpha
                                    prefs.edit().putBoolean("accept_alpha", acceptAlpha).apply()
                                }
                            },
                        )
                    }
                }
            }

            // 更新渠道
            SegmentedSettingsColumn(title = "更新渠道") {
                item {
                    SegmentedSelectableItem(title = "GitHub", selected = true, onClick = {})
                }
            }

            Spacer(modifier = Modifier.height(24.dp))
        }
    }
}
