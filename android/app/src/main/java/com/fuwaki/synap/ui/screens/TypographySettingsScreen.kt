package com.fuwaki.synap.ui.screens

import androidx.compose.foundation.background
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// 将诗词拆分为正文和署名的组合
private val poetryList = listOf(
    "“中国人的性情是总喜欢调和折中的……譬如你说，这屋子太暗，须在这里开一个窗，大家一定不允许的。但如果你主张拆掉屋顶，他们就来调和，愿意开窗了。”" to "—— 鲁迅《无声的中国》",
    "“我翻开历史一查，这历史没有年代，歪歪斜斜的每页上都写着‘仁义道德’四个字。”" to "—— 鲁迅《狂人日记》",
    "“我说道：“爸爸，你走吧。”他往车外看了看，说：“我买几个橘子去。你就在此地，不要走动。”我看那边月台的栅栏外有几个卖东西的等着顾客。走到那边月台，须穿过铁道，须跳下去又爬上去。父亲是一个胖子，走过去自然要费事些。我本来要去的，他不肯，只好让他去。我看见他戴着黑布小帽，穿着黑布大马褂，深青布棉袍，蹒跚地走到铁道边，慢慢探身下去，尚不大难……”" to "—— 朱自清《背影》",
    "“于是我就明白了，他以前那些点头微笑等等等等，全是投资！这就是鲁迅说的“精神的资本家”，投资收获了我的推荐信，然后就“拜拜”了，因为你对他已经没用了。这是一个绝对的利己主义者，他的一切行为，都从利益出发，而且是精心设计，但是他是高智商、高水平，他所做的一切都合理合法”" to "—— 钱理群《大学里绝对精致的利己主义者》",
    "“楼下一个男人病得要死，那间壁的一家唱着留声机；对面是弄孩子。楼上有两人狂笑；还有打牌声。河中的船上有女人哭着她死去的母亲。人类的悲欢并不相通，我只觉得他们吵闹。”" to "—— 鲁迅《而已集·小杂感》",
    "“有的人活着，他已经死了；有的人死了，他还活着。”" to "—— 臧克家《有的人》",
    "在我的后园，可以看见墙外有两株树，一株是枣树，还有一株也是枣树。" to "——鲁迅《秋夜》"
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TypographySettingsScreen(
    currentFontFamily: String,
    onFontFamilyChange: (String) -> Unit,
    noteTextSize: Float,
    onNoteTextSizeChange: (Float) -> Unit,
    onNavigateBack: () -> Unit
) {
    // 随机获取一条诗词数据
    val previewItem = remember { poetryList.random() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("文字样式") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "返回")
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
        ) {
            Spacer(modifier = Modifier.height(8.dp))

            Text(
                text = "字体",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
            )
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant),
            ) {
                listOf(
                    "SansSerif" to "无衬线字体 (默认)",
                    "Serif" to "衬线字体"
                ).forEachIndexed { index, option ->
                    val isSelected = currentFontFamily == option.first
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onFontFamilyChange(option.first) }
                            .padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            // 选项预览也应用对应的字体
                            Text(
                                text = option.second,
                                style = MaterialTheme.typography.bodyLarge,
                                fontFamily = if (option.first == "Serif") FontFamily.Serif else FontFamily.SansSerif,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                        }
                        if (isSelected) {
                            Icon(
                                Icons.Filled.Check,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(24.dp),
                            )
                        }
                    }
                    if (index < 1) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                            modifier = Modifier.padding(horizontal = 16.dp),
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            Text(
                text = "字号",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
            )
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant)
                    .padding(16.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "笔记文字大小 (当前为${noteTextSize.toInt()}sp)",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    TextButton(
                        onClick = { onNoteTextSizeChange(16f) },
                        enabled = noteTextSize != 16f
                    ) {
                        Text("恢复默认")
                    }
                }
                Spacer(modifier = Modifier.height(8.dp))
                Slider(
                    value = noteTextSize,
                    onValueChange = onNoteTextSizeChange,
                    valueRange = 10f..30f,
                    steps = 19
                )

                Spacer(modifier = Modifier.height(24.dp))
                Text(
                    text = "文字大小预览",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(8.dp))

                // 预览框
                Surface(
                    color = MaterialTheme.colorScheme.surface,
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = previewItem.first,
                        fontSize = noteTextSize.sp,
                        lineHeight = noteTextSize.sp * 1.5f,
                        color = MaterialTheme.colorScheme.onSurface,
                        // 动态应用选中的字体
                        fontFamily = if (currentFontFamily == "Serif") FontFamily.Serif else FontFamily.SansSerif,
                        modifier = Modifier.padding(12.dp)
                    )
                }
                Spacer(modifier = Modifier.height(8.dp))

                // 署名
                Text(
                    text = previewItem.second,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    fontFamily = if (currentFontFamily == "Serif") FontFamily.Serif else FontFamily.SansSerif,
                    modifier = Modifier.align(Alignment.End)
                )
            }
        }
    }
}