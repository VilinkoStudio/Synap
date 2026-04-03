package com.fuwaki.synap

import android.content.Context
import android.os.Build
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.os.LocaleListCompat
import androidx.hilt.navigation.compose.hiltViewModel
import com.fuwaki.synap.ui.data.AppLocale // 引入你新建的数据类
import com.fuwaki.synap.ui.data.sampleLanguages
import com.fuwaki.synap.ui.navigation.SynapNavGraph
import com.fuwaki.synap.ui.theme.MyApplicationTheme
import com.fuwaki.synap.ui.viewmodel.AppSessionUiState
import com.fuwaki.synap.ui.viewmodel.AppSessionViewModel

// --- 定义专属于笔记正文的 CompositionLocal ---
val LocalNoteTextSize = compositionLocalOf { 16.sp }
val LocalNoteFontFamily = compositionLocalOf { FontFamily.SansSerif }
val LocalNoteFontWeight = compositionLocalOf { FontWeight.Normal }

@Composable
fun SynapApp(activity: MainActivity?) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("synap_settings", Context.MODE_PRIVATE) }
    val supportsMonet = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S

    var themeMode by remember { mutableIntStateOf(prefs.getInt("themeMode", 0)) }
    var useMonet by remember { mutableStateOf(prefs.getBoolean("useMonet", supportsMonet)) }
    var customThemeHue by remember { mutableFloatStateOf(prefs.getFloat("customThemeHue", 210f)) }

    var selectedLanguageIndex by remember { mutableIntStateOf(prefs.getInt("selectedLanguage", 0)) }
    // --- 修改：获取包含 Tag 的语言列表，并提取出用于 UI 展示的 String 列表 ---
    val baseLanguages = remember { sampleLanguages() }
    val displayLanguages = remember(baseLanguages) {
        listOf("跟随系统语言设置") + baseLanguages.map { it.displayName }
    }

    var noteTextSize by remember { mutableFloatStateOf(prefs.getFloat("noteTextSize", 16f)) }
    var currentFontFamily by remember { mutableStateOf(prefs.getString("fontFamily", "SansSerif") ?: "SansSerif") }
    var currentFontWeight by remember { mutableIntStateOf(prefs.getInt("fontWeight", 400)) }

    var handedness by remember { mutableStateOf(prefs.getString("handedness", "靠右") ?: "靠右") }
    var hasSeenTutorial by remember { mutableStateOf(prefs.getBoolean("hasSeenTutorial", false)) }

    val sessionViewModel: AppSessionViewModel = hiltViewModel()
    val sessionState by sessionViewModel.uiState.collectAsState()

    val isDarkTheme = when (themeMode) {
        1 -> false
        2 -> true
        else -> isSystemInDarkTheme()
    }

    // 计算当前的字体和字重对象
    val actualFontFamily = if (currentFontFamily == "Serif") FontFamily.Serif else FontFamily.SansSerif
    val actualFontWeight = FontWeight(currentFontWeight)

    CompositionLocalProvider(
        LocalNoteTextSize provides noteTextSize.sp,
        LocalNoteFontFamily provides actualFontFamily,
        LocalNoteFontWeight provides actualFontWeight
    ) {
        MyApplicationTheme(darkTheme = isDarkTheme, dynamicColor = supportsMonet && useMonet) {
            val currentScheme = MaterialTheme.colorScheme

            val finalScheme = if (!useMonet) {
                val sPrimary = if (isDarkTheme) 0.6f else 0.8f
                val vPrimary = if (isDarkTheme) 0.8f else 0.45f
                val customPrimary = Color.hsv(customThemeHue, sPrimary, vPrimary)

                val sContainer = if (isDarkTheme) 0.3f else 0.15f
                val vContainer = if (isDarkTheme) 0.25f else 0.95f
                val customPrimaryContainer = Color.hsv(customThemeHue, sContainer, vContainer)
                val sOnContainer = if (isDarkTheme) 0.1f else 0.9f
                val vOnContainer = if (isDarkTheme) 0.9f else 0.1f
                val customOnPrimaryContainer = Color.hsv(customThemeHue, sOnContainer, vOnContainer)

                val sBg = if (isDarkTheme) 0.08f else 0.02f
                val vBg = if (isDarkTheme) 0.08f else 0.99f
                val customBackground = Color.hsv(customThemeHue, sBg, vBg)

                val sVariant = if (isDarkTheme) 0.12f else 0.06f
                val vVariant = if (isDarkTheme) 0.14f else 0.94f
                val customSurfaceVariant = Color.hsv(customThemeHue, sVariant, vVariant)

                currentScheme.copy(
                    primary = customPrimary, onPrimary = if (isDarkTheme) Color(0xFF202020) else Color.White,
                    primaryContainer = customPrimaryContainer, onPrimaryContainer = customOnPrimaryContainer,
                    secondaryContainer = customPrimaryContainer, onSecondaryContainer = customOnPrimaryContainer,
                    background = customBackground, surface = customBackground, surfaceVariant = customSurfaceVariant
                )
            } else {
                currentScheme
            }

            MaterialTheme(colorScheme = finalScheme, typography = Typography(), shapes = MaterialTheme.shapes) {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    when (val state = sessionState) {
                        AppSessionUiState.Initializing -> SessionLoadingScreen()
                        is AppSessionUiState.Error -> SessionErrorScreen(message = state.message, onRetry = sessionViewModel::initialize)
                        AppSessionUiState.Ready -> SynapNavGraph(
                            themeMode = themeMode, onThemeModeChange = { themeMode = it; prefs.edit().putInt("themeMode", it).apply() },
                            useMonet = useMonet, supportsMonet = supportsMonet, onUseMonetChange = { useMonet = it; prefs.edit().putBoolean("useMonet", it).apply() },
                            customThemeHue = customThemeHue, onCustomThemeHueChange = { customThemeHue = it; prefs.edit().putFloat("customThemeHue", it).apply() },
                            handedness = handedness, onHandednessChange = { handedness = it; prefs.edit().putString("handedness", it).apply() },
                            // --- 修改：将显示用的 String 列表传给 UI，并执行语言切换逻辑 ---
                            languages = displayLanguages,
                            selectedLanguageIndex = selectedLanguageIndex,
                            onLanguageSelect = { index ->
                                selectedLanguageIndex = index
                                prefs.edit().putInt("selectedLanguage", index).apply()

                                // 根据索引切换语言
                                if (index == 0) {
                                    AppCompatDelegate.setApplicationLocales(LocaleListCompat.getEmptyLocaleList())
                                } else {
                                    val tag = baseLanguages[index - 1].tag
                                    AppCompatDelegate.setApplicationLocales(LocaleListCompat.forLanguageTags(tag))
                                }
                            },
                            currentFontFamily = currentFontFamily, onFontFamilyChange = { currentFontFamily = it; prefs.edit().putString("fontFamily", it).apply() },
                            currentFontWeight = currentFontWeight, onFontWeightChange = { currentFontWeight = it; prefs.edit().putInt("fontWeight", it).apply() },
                            noteTextSize = noteTextSize, onNoteTextSizeChange = { noteTextSize = it; prefs.edit().putFloat("noteTextSize", it).apply() },
                            hasSeenTutorial = hasSeenTutorial, onTutorialFinished = { hasSeenTutorial = true; prefs.edit().putBoolean("hasSeenTutorial", true).apply() },
                            databaseActivity = activity,
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun SessionLoadingScreen() {
    Column(modifier = Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
        CircularProgressIndicator()
        Text("正在初始化 Synap...", modifier = Modifier.padding(top = 16.dp), style = MaterialTheme.typography.bodyLarge)
    }
}

@Composable
fun SessionErrorScreen(message: String, onRetry: () -> Unit) {
    Column(modifier = Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
        Text("启动失败", style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.error)
        Text(message, modifier = Modifier.padding(top = 12.dp), style = MaterialTheme.typography.bodyMedium)
        Button(onClick = onRetry, modifier = Modifier.padding(top = 20.dp)) { Text("重试") }
    }
}