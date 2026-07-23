package com.synap.app.ui.screens

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.R
import kotlinx.coroutines.CancellationException

// 1. 定义数据结构（增加可选的 platformNameRes 用于多语言支持）
data class SocialLink(val platformName: String, val platformNameRes: Int? = null, val url: String)
data class TeamMember(val name: String, val socialLinks: List<SocialLink>)

// 2. 团队成员列表
val creativeTeamList = listOf(
    TeamMember(
        name = "Fuwaki",
        socialLinks = listOf(
            // 为 bilibili 绑定字符串资源 ID，支持多语言适配
            SocialLink("bilibili", R.string.bilibili, "https://space.bilibili.com/488218512"),
            SocialLink("GitHub", null, "https://github.com/Fuwaki")
        )
    ),
    TeamMember(
        name = "尧尧切克Now",
        socialLinks = listOf(
            SocialLink("Blog", null, "https://yyckn.rth1.xyz/"),
            SocialLink("GitHub", null, "https://github.com/yyckn")
        )
    ),
    TeamMember(
        name = "Kitra",
        socialLinks = listOf(
            SocialLink("bilibili", R.string.bilibili, "https://space.bilibili.com/180371610"),
            SocialLink("Blog", null, "https://blog.kitramgp.cn/"),
            SocialLink("GitHub", null, "https://github.com/KitraMGP")
        )
    )
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TeamScreen(onNavigateBack: () -> Unit) {
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
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.creative_team)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                }
            )
        }
    ) { innerPadding ->
        LazyColumn(
            contentPadding = PaddingValues(
                top = innerPadding.calculateTopPadding() + 16.dp,
                bottom = innerPadding.calculateBottomPadding() + 16.dp,
                start = 16.dp,
                end = 16.dp
            ),
            modifier = Modifier.fillMaxSize(),
        ) {
            item {
                Text(
                    text = stringResource(R.string.creative_team),
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
                )
            }

            item {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(16.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant)
                ) {
                    creativeTeamList.forEachIndexed { index, member ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = member.name,
                                style = MaterialTheme.typography.bodyLarge,
                                color = MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.weight(1f),
                            )
                            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                member.socialLinks.forEach { link ->
                                    val platformText = link.platformNameRes?.let { stringResource(it) } ?: link.platformName
                                    Button(
                                        onClick = {
                                            try {
                                                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link.url)))
                                            } catch (e: Exception) {
                                                e.printStackTrace()
                                            }
                                        },
                                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 0.dp),
                                        modifier = Modifier.height(28.dp),
                                    ) {
                                        Text(text = platformText, fontSize = 11.sp)
                                    }
                                }
                            }
                        }
                        if (index < creativeTeamList.size - 1) {
                            HorizontalDivider(
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                                modifier = Modifier.padding(horizontal = 16.dp)
                            )
                        }
                    }
                }
            }

            item {
                Spacer(modifier = Modifier.height(24.dp))
            }

            item {
                Text(
                    text = stringResource(R.string.publisher),
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
                )
            }

            item {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(16.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant)
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "厦门市同安区云上幻可网络科技工作室",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                }
            }

            // ========== 新增：赞助二维码区域 ==========
            item {
                // 【核心修复】：通过判断当前界面的实际背景亮度来判断深浅色
                // 而不是依赖 isSystemInDarkTheme() (它会忽略 App 内强制切换的主题)
                val isDark = MaterialTheme.colorScheme.surface.luminance() < 0.5f

                // 根据当前的 UI 主题状态动态切换图片资源
                val qrImageRes = if (isDark) R.drawable.sponsor_qr_dark else R.drawable.sponsor_qr_light

                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 32.dp, bottom = 24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Text(
                        text = "赞助开发者",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.primary
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "如果 Synap 对您有帮助，可以请我们喝杯咖啡~",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(24.dp))

                    // 给二维码加上好看的卡片底板
                    Surface(
                        shape = RoundedCornerShape(24.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        tonalElevation = 2.dp,
                        shadowElevation = 4.dp
                    ) {
                        Image(
                            painter = painterResource(id = qrImageRes),
                            contentDescription = "赞助二维码",
                            modifier = Modifier
                                .size(240.dp)
                                .padding(16.dp)
                                .clip(RoundedCornerShape(12.dp))
                        )
                    }
                }
            }
        }
    }
}