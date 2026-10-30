package com.synap.app.ui.screens

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
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
import com.synap.app.ui.util.navigationBarsScrollPadding
import com.synap.app.ui.components.SegmentedContentItem
import com.synap.app.ui.components.SegmentedSettingsColumn
import kotlinx.coroutines.CancellationException

data class SocialLink(val platformName: String, val platformNameRes: Int? = null, val url: String)
data class TeamMember(val name: String, val socialLinks: List<SocialLink>)

val creativeTeamList = listOf(
    TeamMember(
        name = "Fuwaki",
        socialLinks = listOf(
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
                title = { Text(stringResource(R.string.creative_team)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 16.dp)
                .navigationBarsScrollPadding(),
        ) {
            SegmentedSettingsColumn(title = stringResource(R.string.creative_team)) {
                creativeTeamList.forEach { member ->
                    item {
                        SegmentedContentItem {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
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
                                                runCatching {
                                                    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link.url)))
                                                }
                                            },
                                            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 12.dp, vertical = 0.dp),
                                            modifier = Modifier.height(28.dp),
                                        ) {
                                            Text(text = platformText, fontSize = 11.sp)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            SegmentedSettingsColumn(title = stringResource(R.string.publisher)) {
                item {
                    SegmentedContentItem {
                        Text(
                            text = "厦门市同安区云上幻可网络科技工作室",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(32.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 24.dp),
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

                val isDark = MaterialTheme.colorScheme.surface.luminance() < 0.5f
                val qrImageRes = if (isDark) R.drawable.sponsor_qr_dark else R.drawable.sponsor_qr_light

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
