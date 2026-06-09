package com.synap.app.ui.screens

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.synap.app.R
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
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Spacer(modifier = Modifier.height(4.dp))

            // 版本信息
            Text(
                text = "版本信息",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(start = 4.dp),
            )
            Surface(
                modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)),
                color = MaterialTheme.colorScheme.surfaceVariant,
            ) {
                Column {
                    // 当前版本
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "当前版本",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            maxLines = 1,
                        )
                        Spacer(modifier = Modifier.width(16.dp))
                        Text(
                            text = buildVersion + (buildVersionDetails?.takeIf { it.isNotBlank() }?.let { " · $it" } ?: ""),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.weight(1f),
                            textAlign = TextAlign.End,
                        )
                    }

                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )

                    // 类型
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "类型",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.weight(1f),
                        )
                        val typeLabel = when (versionType) {
                            VersionType.Release -> "正式版"
                            VersionType.Beta -> "Beta 测试版"
                            VersionType.Alpha -> "Alpha 测试版"
                            VersionType.Dev -> "Developer 开发版本"
                        }
                        Text(
                            text = typeLabel,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            // 更新选项
            Text(
                text = "更新选项",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(start = 4.dp),
            )
            Surface(
                modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)),
                color = MaterialTheme.colorScheme.surfaceVariant,
            ) {
                Column {
                    // 自动检测更新
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                if (versionType != VersionType.Dev) {
                                    autoCheck = !autoCheck
                                    prefs.edit().putBoolean("auto_check_update", autoCheck).apply()
                                }
                            }
                            .padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = "自动检测新版本",
                                style = MaterialTheme.typography.bodyLarge,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                            if (versionType == VersionType.Dev) {
                                Text(
                                    text = "Debug版本无法进行自动检测，请切换到已构建版本后重试",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        Switch(
                            checked = autoCheck,
                            onCheckedChange = {
                                if (versionType != VersionType.Dev) {
                                    autoCheck = it
                                    prefs.edit().putBoolean("auto_check_update", it).apply()
                                }
                            },
                            enabled = versionType != VersionType.Dev,
                        )
                    }

                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )

                    // 接收哪些版本的更新
                    Column {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { isChannelExpanded = !isChannelExpanded }
                                .padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = "接收哪些版本的更新",
                                    style = MaterialTheme.typography.bodyLarge,
                                    color = MaterialTheme.colorScheme.onSurface,
                                )
                                val selected = buildList {
                                    if (acceptRelease) add("Release")
                                    if (acceptBeta) add("Beta")
                                    if (acceptAlpha) add("Alpha")
                                }.joinToString("，")
                                Text(
                                    text = selected.ifEmpty { "未选择" },
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            Icon(
                                imageVector = if (isChannelExpanded) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }

                        AnimatedVisibility(
                            visible = isChannelExpanded,
                            enter = expandVertically(),
                            exit = shrinkVertically(),
                        ) {
                            Column(modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 16.dp)) {
                                ChannelCheckbox("Release", acceptRelease) {
                                    if (!it && !acceptBeta && !acceptAlpha) return@ChannelCheckbox
                                    acceptRelease = it
                                    prefs.edit().putBoolean("accept_release", it).apply()
                                }
                                ChannelCheckbox("Beta", acceptBeta) {
                                    if (!it && !acceptRelease && !acceptAlpha) return@ChannelCheckbox
                                    acceptBeta = it
                                    prefs.edit().putBoolean("accept_beta", it).apply()
                                }
                                ChannelCheckbox("Alpha", acceptAlpha) {
                                    if (!it && !acceptRelease && !acceptBeta) return@ChannelCheckbox
                                    acceptAlpha = it
                                    prefs.edit().putBoolean("accept_alpha", it).apply()
                                }
                            }
                        }
                    }
                }
            }

            // 更新渠道
            Text(
                text = "更新渠道",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(start = 4.dp),
            )
            Surface(
                modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)),
                color = MaterialTheme.colorScheme.surfaceVariant,
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(selected = true, onClick = null)
                    Spacer(modifier = Modifier.width(12.dp))
                    Text(
                        text = "GitHub",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            Spacer(modifier = Modifier.height(24.dp))
        }
    }
}

@Composable
private fun ChannelCheckbox(label: String, checked: Boolean, onCheckedChange: (Boolean) -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onCheckedChange(!checked) }
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Checkbox(checked = checked, onCheckedChange = onCheckedChange)
        Spacer(modifier = Modifier.width(8.dp))
        Text(text = label, style = MaterialTheme.typography.bodyMedium)
    }
}
