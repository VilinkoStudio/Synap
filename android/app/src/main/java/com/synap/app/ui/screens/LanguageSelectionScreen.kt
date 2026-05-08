package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
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
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.synap.app.R
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LanguageSelectionScreen(
    languages: List<String>,
    selectedIndex: Int,
    onLanguageSelect: (Int) -> Unit,
    onNavigateBack: () -> Unit,
) {
    // ========== 预返回手势核心状态 ==========
    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress // 收集滑动进度
            }
            onNavigateBack() // 手指松开且决定返回时，触发导航
        } catch (e: CancellationException) {
            backProgress = 0f // 用户取消了返回手势，重置进度
        }
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
                title = { Text(stringResource(R.string.select_language)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
            )
        },
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 16.dp)
                .padding(bottom = 16.dp)
                .clip(RoundedCornerShape(16.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant),
        ) {
            itemsIndexed(languages) { index, language ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { onLanguageSelect(index) }
                        .padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(text = language, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                    if (selectedIndex == index) {
                        Icon(Icons.Filled.Check, contentDescription = stringResource(R.string.selected), tint = MaterialTheme.colorScheme.primary)
                    }
                }
                if (index < languages.lastIndex) {
                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )
                }
            }
        }
    }
}