package com.synap.app.ui.screens

import android.content.ComponentName
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingLogoScreen(onNavigateBack: () -> Unit) {
    val context = LocalContext.current
    val packageManager = context.packageManager
    val packageName = context.packageName

    // 动态检查当前系统的哪个 Component 处于启用状态
    val oldComponent = ComponentName(context, "$packageName.MainActivityOld")
    val isOldIconCurrentlyEnabled = packageManager.getComponentEnabledSetting(oldComponent) == PackageManager.COMPONENT_ENABLED_STATE_ENABLED

    var selectedIndex by remember { mutableIntStateOf(if (isOldIconCurrentlyEnabled) 1 else 0) }

    // ========== 预返回手势核心状态 ==========
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress // 收集滑动进度 (0.0 ~ 1.0)
            }
            onNavigateBack() // 手指松开且决定返回时触发导航
        } catch (e: CancellationException) {
            backProgress = 0f // 用户取消返回，重置进度
        }
    }

    // 切换图标逻辑
    fun switchIcon(index: Int) {
        if (selectedIndex == index) return
        selectedIndex = index
        val defaultComponent = ComponentName(context, "$packageName.MainActivity")

        if (index == 1) {
            // 启用旧图标，禁用默认
            packageManager.setComponentEnabledSetting(oldComponent, PackageManager.COMPONENT_ENABLED_STATE_ENABLED, PackageManager.DONT_KILL_APP)
            packageManager.setComponentEnabledSetting(defaultComponent, PackageManager.COMPONENT_ENABLED_STATE_DISABLED, PackageManager.DONT_KILL_APP)
        } else {
            // 启用默认，禁用旧图标
            packageManager.setComponentEnabledSetting(defaultComponent, PackageManager.COMPONENT_ENABLED_STATE_ENABLED, PackageManager.DONT_KILL_APP)
            packageManager.setComponentEnabledSetting(oldComponent, PackageManager.COMPONENT_ENABLED_STATE_DISABLED, PackageManager.DONT_KILL_APP)
        }
        Toast.makeText(context, "图标已切换，返回桌面后系统会自动刷新", Toast.LENGTH_SHORT).show()
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            // ========== 应用预返回手势的视觉形变 ==========
            .graphicsLayer {
                translationX = backProgress * 64.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
                shape = RoundedCornerShape(32.dp * backProgress) // 随进度增加圆角
                clip = true
            },
        topBar = {
            TopAppBar(
                title = { Text("应用图标") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(16.dp)
        ) {
            Text(
                text = "桌面图标样式",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
            )

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant)
            ) {
                val options = listOf("Synap彩色图标", "莫奈取色图标")

                options.forEachIndexed { index, title ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { switchIcon(index) }
                            .padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = title,
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.weight(1f)
                        )
                        if (selectedIndex == index) {
                            Icon(
                                Icons.Filled.Check,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(24.dp)
                            )
                        }
                    }
                    if (index < options.size - 1) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                            modifier = Modifier.padding(horizontal = 16.dp)
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Text(
                text = "注意：由于系统限制，切换图标后可能会出现闪退到桌面的情况。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 8.dp)
            )
        }
    }
}