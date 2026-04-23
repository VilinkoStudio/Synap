package com.synap.app.ui.screens

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.R

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

    Scaffold(
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
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            items(creativeTeamList) { member ->
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = RoundedCornerShape(16.dp)
                ) {
                    Row(
                        modifier = Modifier
                            .padding(16.dp)
                            .fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        // 左侧：仅保留姓名
                        Text(
                            text = member.name,
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.weight(1f)
                        )

                        Spacer(modifier = Modifier.width(16.dp))

                        // 右侧：社交平台按钮组
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            member.socialLinks.forEach { link ->
                                Button(
                                    onClick = {
                                        try {
                                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link.url)))
                                        } catch (e: Exception) {
                                            e.printStackTrace() // 防止链接格式错误导致崩溃
                                        }
                                    },
                                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 0.dp),
                                    modifier = Modifier.height(32.dp)
                                ) {
                                    // 优先使用 stringResource 翻译，如果没有绑定资源 ID 则显示原始 platformName
                                    val platformText = link.platformNameRes?.let { stringResource(it) } ?: link.platformName
                                    Text(text = platformText, fontSize = 12.sp)
                                }
                            }
                        }
                    }
                }
            }

            // ========== 新增：赞助二维码区域 ==========
            item {
                val isDark = isSystemInDarkTheme()
                // 根据深浅色模式动态切换图片资源
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