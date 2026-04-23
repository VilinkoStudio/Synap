package com.synap.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.FormatListBulleted
import androidx.compose.material.icons.automirrored.filled.Reply
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.FormatBold
import androidx.compose.material.icons.filled.FormatColorText
import androidx.compose.material.icons.filled.FormatItalic
import androidx.compose.material.icons.filled.FormatQuote
import androidx.compose.material.icons.filled.FormatStrikethrough
import androidx.compose.material.icons.filled.FormatUnderlined
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FloatingToolbarDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.HorizontalFloatingToolbar
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.InputChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.synap.app.R
import kotlinx.coroutines.delay

@Composable
fun TutorialScreen(
    currentThemeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    availableLanguages: List<String>,
    currentLanguageIndex: Int,
    onLanguageSelect: (Int) -> Unit,
    onFinishTutorial: () -> Unit
) {
    // 控制当前所处页面（0=欢迎页，1=卡片手势页，2=新建笔记页）
    var currentPage by remember { mutableIntStateOf(0) }

    Scaffold { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            when (currentPage) {
                0 -> IntroPage(
                    currentThemeMode = currentThemeMode,
                    onThemeModeChange = onThemeModeChange,
                    useMonet = useMonet,
                    supportsMonet = supportsMonet,
                    onUseMonetChange = onUseMonetChange,
                    customThemeHue = customThemeHue,
                    onCustomThemeHueChange = onCustomThemeHueChange,
                    availableLanguages = availableLanguages,
                    currentLanguageIndex = currentLanguageIndex,
                    onLanguageSelect = onLanguageSelect,
                    onNext = { currentPage = 1 },
                    onSkip = onFinishTutorial
                )
                1 -> NoteCardTutorialPage()
                2 -> NewNoteTutorialPage()
            }

            // 底部导航栏 (仅在第二屏及之后显示，且移除了跳过按钮)
            if (currentPage > 0) {
                Column(
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = 32.dp, start = 16.dp, end = 16.dp)
                        .fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Button(onClick = { currentPage -= 1 }) {
                            Text("上一个")
                        }
                        Button(onClick = {
                            if (currentPage == 2) {
                                onFinishTutorial()
                            } else {
                                currentPage += 1
                            }
                        }) {
                            Text(if (currentPage == 2) "完成" else "下一个")
                        }
                    }
                }
            }
        }
    }
}

// 第一屏：欢迎介绍与真实生效的快速设置
@Composable
private fun IntroPage(
    currentThemeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    availableLanguages: List<String>,
    currentLanguageIndex: Int,
    onLanguageSelect: (Int) -> Unit,
    onNext: () -> Unit,
    onSkip: () -> Unit
) {
    var showLanguageDialog by remember { mutableStateOf(false) }

    if (showLanguageDialog) {
        AlertDialog(
            onDismissRequest = { showLanguageDialog = false },
            title = { Text(stringResource(R.string.select_language)) },
            text = {
                Column {
                    availableLanguages.forEachIndexed { index, lang ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    onLanguageSelect(index)
                                    showLanguageDialog = false
                                }
                                .padding(vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(lang, modifier = Modifier.weight(1f))
                            if (currentLanguageIndex == index) {
                                Icon(Icons.Filled.Check, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { showLanguageDialog = false }) { Text("关闭") }
            }
        )
    }

    Column(
        modifier = Modifier.fillMaxSize()
    ) {
        // 上半部分：滚动区域，包含标题和设置
        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(start = 24.dp, end = 24.dp, top = 48.dp)
        ) {
            Text(
                text = "Synap",
                style = MaterialTheme.typography.displayLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary,
                textAlign = TextAlign.Start
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "一款极简的用于快速思维捕获的笔记应用",
                style = MaterialTheme.typography.titleMedium,
                textAlign = TextAlign.Start,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Spacer(modifier = Modifier.height(48.dp))

            // 快速设置区域
            Text(
                text = "快速设置",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.primary,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "您也可以稍后在设置页中调整这些选项",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(16.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant)
            ) {
                // 1. 深浅色
                val themeOptions = listOf(
                    stringResource(R.string.theme_system),
                    stringResource(R.string.theme_light),
                    stringResource(R.string.theme_dark)
                )

                themeOptions.forEachIndexed { index, title ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onThemeModeChange(index) }
                            .padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(title, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface, modifier = Modifier.weight(1f))
                        if (currentThemeMode == index) {
                            Icon(Icons.Filled.Check, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                        }
                    }
                    if (index < 2) HorizontalDivider(color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f), modifier = Modifier.padding(horizontal = 16.dp))
                }

                HorizontalDivider(color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f), modifier = Modifier.padding(horizontal = 16.dp))

                // 2. Monet 开关
                Row(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(Icons.Filled.Palette, null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(end = 16.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.sync_system_color), style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
                        Text(
                            text = when {
                                !supportsMonet -> stringResource(R.string.monet_unsupported)
                                useMonet -> stringResource(R.string.monet_enabled)
                                else -> stringResource(R.string.monet_disabled)
                            },
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    Switch(checked = useMonet, onCheckedChange = onUseMonetChange, enabled = supportsMonet)
                }

                // 3. 拖动调节主题色
                AnimatedVisibility(visible = !useMonet, enter = expandVertically() + fadeIn(), exit = shrinkVertically() + fadeOut()) {
                    Column {
                        HorizontalDivider(color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f), modifier = Modifier.padding(horizontal = 16.dp))
                        Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Filled.Tune, null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(end = 16.dp))
                                Text(stringResource(R.string.adjust_theme_color), style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
                            }
                            Spacer(modifier = Modifier.height(8.dp))
                            val currentPureColor = Color.hsv(customThemeHue, 1f, 1f)
                            Slider(
                                value = customThemeHue,
                                onValueChange = onCustomThemeHueChange,
                                valueRange = 0f..360f,
                                colors = SliderDefaults.colors(thumbColor = currentPureColor, activeTrackColor = currentPureColor)
                            )
                        }
                    }
                }

                HorizontalDivider(color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f), modifier = Modifier.padding(horizontal = 16.dp))

                // 4. 语言选择
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { showLanguageDialog = true }
                        .padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(Icons.Filled.Language, null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(end = 16.dp))
                    Text(stringResource(R.string.language), style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface, modifier = Modifier.weight(1f))
                    Text(availableLanguages.getOrNull(currentLanguageIndex) ?: "", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }

        // 下半部分：固定在底部的按钮
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Button(onClick = onNext) {
                Text("查看使用教程")
            }
            Spacer(modifier = Modifier.height(8.dp))
            TextButton(onClick = onSkip) {
                Text("我已了解使用方法，跳过教程")
            }
        }
    }
}

// 第二屏：卡片手势与多选动画教程
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun NoteCardTutorialPage() {
    val offsetX = remember { Animatable(0f) }
    val scale = remember { Animatable(1f) }
    var hintText by remember { mutableStateOf("右滑笔记可以删除") }
    var isMultiSelect by remember { mutableStateOf(false) }

    // 核心自动演示动画循环
    LaunchedEffect(Unit) {
        while (true) {
            // 1. 演示右滑删除
            hintText = "右滑笔记可以删除"
            delay(800)
            offsetX.animateTo(150f, animationSpec = tween(500))
            delay(1200)
            offsetX.animateTo(0f, animationSpec = tween(500))
            delay(800)

            // 2. 演示左滑回复
            hintText = "左滑笔记可以回复这条笔记"
            offsetX.animateTo(-150f, animationSpec = tween(500))
            delay(1200)
            offsetX.animateTo(0f, animationSpec = tween(500))
            delay(800)

            // 3. 演示长按多选
            hintText = "长按笔记可以触发多选"
            // 模拟手指按下缩放
            scale.animateTo(0.95f, tween(150))
            delay(300)
            scale.animateTo(1f, tween(150))

            isMultiSelect = true
            delay(3000)

            // 恢复初始状态
            isMultiSelect = false
            delay(1000)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp)
    ) {
        Spacer(modifier = Modifier.height(32.dp))
        Text(
            text = "笔记卡片",
            style = MaterialTheme.typography.headlineLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.primary
        )

        Spacer(modifier = Modifier.height(80.dp))

        Box(
            modifier = Modifier.fillMaxWidth(),
            contentAlignment = Alignment.Center
        ) {
            // 底层滑动背景：右滑显示左侧(红/删除)，左滑显示右侧(主色/回复)
            val bgColor = when {
                offsetX.value > 0 -> MaterialTheme.colorScheme.errorContainer
                offsetX.value < 0 -> MaterialTheme.colorScheme.primaryContainer
                else -> Color.Transparent
            }

            Box(
                modifier = Modifier
                    .matchParentSize()
                    .background(bgColor, RoundedCornerShape(12.dp))
                    .padding(horizontal = 24.dp)
            ) {
                if (offsetX.value > 0) {
                    Icon(
                        imageVector = Icons.Filled.Delete,
                        contentDescription = "删除",
                        modifier = Modifier.align(Alignment.CenterStart), // 卡片向右滑，图标在左边露出来
                        tint = MaterialTheme.colorScheme.onErrorContainer
                    )
                } else if (offsetX.value < 0) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.Reply,
                        contentDescription = "回复",
                        modifier = Modifier.align(Alignment.CenterEnd), // 卡片向左滑，图标在右边露出来
                        tint = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }
            }

            // 顶层示例卡片
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .offset(x = offsetX.value.dp) // 跟随动画滑动
                    .scale(scale.value),          // 跟随动画缩放
                colors = CardDefaults.cardColors(
                    containerColor = if (isMultiSelect) MaterialTheme.colorScheme.secondaryContainer
                    else MaterialTheme.colorScheme.surfaceVariant
                ),
                shape = RoundedCornerShape(12.dp)
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "这是一条示例笔记，仅用作教程使用。千门万户曈曈日，总把新桃换旧符。",
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.weight(1f)
                    )

                    // 多选状态下的复选框
                    AnimatedVisibility(visible = isMultiSelect) {
                        Checkbox(
                            checked = true,
                            onCheckedChange = null,
                            modifier = Modifier.padding(start = 16.dp)
                        )
                    }
                }
            }
        }

        // 多选模式下紧贴卡片下方出现的浮动工具栏
        AnimatedVisibility(
            visible = isMultiSelect,
            enter = fadeIn() + slideInVertically(initialOffsetY = { -it }),
            exit = fadeOut() + slideOutVertically(targetOffsetY = { -it }),
            modifier = Modifier.align(Alignment.CenterHorizontally).padding(top = 16.dp)
        ) {
            HorizontalFloatingToolbar(
                expanded = true,
                colors = FloatingToolbarDefaults.standardFloatingToolbarColors(
                    toolbarContainerColor = MaterialTheme.colorScheme.primaryContainer,
                    toolbarContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                )
            ) {
                IconButton(onClick = { }) { Icon(Icons.Filled.ContentCopy, contentDescription = "复制") }
                IconButton(onClick = { }) { Icon(Icons.Filled.Share, contentDescription = "分享") }
                IconButton(onClick = { }) { Icon(Icons.Filled.Delete, contentDescription = "删除") }
            }
        }

        Spacer(modifier = Modifier.weight(1f))

        // 下方动态变化的提示文字
        Text(
            text = hintText,
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.fillMaxWidth().padding(bottom = 80.dp),
            textAlign = TextAlign.Center
        )
    }
}

// 第三屏：新建笔记动画教程
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun NewNoteTutorialPage() {
    var hintText by remember { mutableStateOf("") }
    var noteText by remember { mutableStateOf("") }
    var tagInputText by remember { mutableStateOf("") }

    var showRecommendedTag by remember { mutableStateOf(false) }
    var isAddingTag by remember { mutableStateOf(false) }
    var showToolbar by remember { mutableStateOf(false) }
    var clickTarget by remember { mutableStateOf("") }

    var selectedTags by remember { mutableStateOf<List<String>>(emptyList()) }

    // 核心自动演示动画循环
    LaunchedEffect(Unit) {
        while (true) {
            // 初始状态清理
            noteText = ""
            tagInputText = ""
            selectedTags = emptyList()
            showRecommendedTag = false
            isAddingTag = false
            showToolbar = false
            clickTarget = ""

            // 1. 提示新建
            hintText = "点击首页的“新建笔记”按钮开始记笔记"
            delay(2000)

            // 2. 兼容Markdown介绍
            hintText = "兼容Markdown语法的编辑器"
            delay(1000)
            showToolbar = true
            delay(1500)

            // 模拟点击工具栏加粗（“**”）
            clickTarget = "toolbar_bold"
            delay(400)
            clickTarget = ""
            noteText = "****"
            delay(800)

            // 3. 打字机输入
            val fullNote = "千门万户曈曈日，总把新桃换旧符。"
            for (i in 1..fullNote.length) {
                noteText = "**" + fullNote.substring(0, i) + "**"
                delay(120) // 打字速度
            }
            delay(1000)

            // 4. 模拟出现推荐标签
            hintText = "Synap可以根据你所输入的内容推荐标签"
            showRecommendedTag = true
            delay(2000)

            // 模拟点击推荐标签
            clickTarget = "rec_tag"
            delay(400)
            clickTarget = ""
            showRecommendedTag = false
            selectedTags = listOf("古诗")
            delay(1000)

            // 5. 模拟新建标签
            hintText = "点击“新建标签”可以添加自定义标签"
            delay(1000)
            clickTarget = "add_tag"
            delay(400)
            clickTarget = ""
            isAddingTag = true
            delay(500)

            // 模拟打字输入新标签
            val fullTag = "王安石"
            for (i in 1..fullTag.length) {
                tagInputText = fullTag.substring(0, i)
                delay(150)
            }
            delay(800)

            // 模拟点击确定
            clickTarget = "check_tag"
            delay(400)
            clickTarget = ""
            isAddingTag = false
            tagInputText = ""
            selectedTags = listOf("古诗", "王安石")

            delay(3500)
        }
    }

    Box(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(top = 48.dp) // 避开顶部空间
        ) {
            Text(
                text = "新建笔记",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(horizontal = 24.dp)
            )

            Spacer(modifier = Modifier.height(24.dp))

            // 模拟的新建笔记 UI
            Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) {

                // 标签栏区域
                if (isAddingTag) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        OutlinedTextField(
                            value = tagInputText,
                            onValueChange = {},
                            placeholder = { Text("输入标签") },
                            modifier = Modifier.weight(1f).height(56.dp),
                            singleLine = true,
                            readOnly = true // 教程演示专用
                        )
                        Box(contentAlignment = Alignment.Center) {
                            IconButton(onClick = {}) {
                                Icon(Icons.Filled.Check, contentDescription = "确认添加", tint = MaterialTheme.colorScheme.primary)
                            }
                            ClickPointer(visible = clickTarget == "check_tag")
                        }
                    }
                } else {
                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        selectedTags.forEach { tag ->
                            InputChip(
                                selected = true,
                                onClick = {},
                                label = { Text(tag) },
                                trailingIcon = { Icon(Icons.Filled.Close, null, Modifier.size(InputChipDefaults.AvatarSize)) }
                            )
                        }
                        Box(contentAlignment = Alignment.Center) {
                            InputChip(
                                selected = false,
                                onClick = {},
                                label = { Text("添加标签") },
                                trailingIcon = { Icon(Icons.Filled.Add, null, Modifier.size(16.dp)) }
                            )
                            ClickPointer(visible = clickTarget == "add_tag")
                        }
                    }
                }

                // 推荐标签区域
                if (showRecommendedTag) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(top = 4.dp, bottom = 8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("推荐标签：", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Box(contentAlignment = Alignment.Center) {
                            Text(
                                text = "#古诗",
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.clip(RoundedCornerShape(4.dp)).background(MaterialTheme.colorScheme.primaryContainer.copy(alpha=0.3f)).padding(horizontal = 4.dp, vertical = 2.dp)
                            )
                            ClickPointer(visible = clickTarget == "rec_tag")
                        }
                    }
                }

                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

                // 输入区域 (高度缩短至3行左右)
                Box(modifier = Modifier.fillMaxWidth().height(80.dp)) {
                    Text(
                        text = noteText.ifEmpty { "开始记录你的灵感..." },
                        style = MaterialTheme.typography.bodyLarge,
                        color = if (noteText.isEmpty()) MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f) else MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.padding(top = 8.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            // 模拟工具栏悬浮在提示词上方
            AnimatedVisibility(
                visible = showToolbar,
                enter = fadeIn() + slideInVertically(initialOffsetY = { 20 }),
                exit = fadeOut() + slideOutVertically(targetOffsetY = { 20 }),
                modifier = Modifier.padding(bottom = 24.dp)
            ) {
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.surface,
                    tonalElevation = 3.dp,
                    shadowElevation = 8.dp
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 8.dp, vertical = 8.dp)
                            .horizontalScroll(rememberScrollState()),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        val iconColor = MaterialTheme.colorScheme.onSurface
                        val textStyle = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold, color = iconColor)

                        // 在加粗按钮上设置模拟点击锚点
                        Box(contentAlignment = Alignment.Center) {
                            IconButton(onClick = {}) { Icon(Icons.Filled.FormatBold, null, tint = iconColor) }
                            ClickPointer(visible = clickTarget == "toolbar_bold")
                        }
                        IconButton(onClick = {}) { Icon(Icons.Filled.FormatItalic, null, tint = iconColor) }
                        IconButton(onClick = {}) { Icon(Icons.Filled.FormatStrikethrough, null, tint = iconColor) }
                        IconButton(onClick = {}) { Icon(Icons.Filled.FormatUnderlined, null, tint = iconColor) }
                        IconButton(onClick = {}) { Icon(Icons.Filled.FormatColorText, null, tint = iconColor) }
                        IconButton(onClick = {}) { Icon(Icons.Filled.FormatQuote, null, tint = iconColor) }
                        IconButton(onClick = {}) { Text("H1", style = textStyle) }
                        IconButton(onClick = {}) { Text("H2", style = textStyle) }
                        IconButton(onClick = {}) { Icon(Icons.AutoMirrored.Filled.FormatListBulleted, null, tint = iconColor) }
                        IconButton(onClick = {}) { Text("1.", style = textStyle) }
                    }
                }
            }

            // 提示词
            Text(
                text = hintText,
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.fillMaxWidth().padding(bottom = 120.dp),
                textAlign = TextAlign.Center
            )
        }
    }
}

// 模拟点击时的视觉圆点反馈
@Composable
private fun ClickPointer(visible: Boolean) {
    AnimatedVisibility(
        visible = visible,
        enter = fadeIn(tween(150)),
        exit = fadeOut(tween(300))
    ) {
        Box(
            modifier = Modifier
                .size(40.dp)
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.4f), CircleShape)
        )
    }
}