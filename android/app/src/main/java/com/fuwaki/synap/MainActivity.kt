package com.fuwaki.synap

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Build
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import androidx.hilt.navigation.compose.hiltViewModel
import com.fuwaki.synap.data.service.SynapServiceApi
import com.fuwaki.synap.ui.data.sampleLanguages
import com.fuwaki.synap.ui.screens.HomeScreen
import com.fuwaki.synap.ui.screens.LanguageSelectionScreen
import com.fuwaki.synap.ui.screens.NewNoteScreen
import com.fuwaki.synap.ui.screens.NoteDetailScreen
import com.fuwaki.synap.ui.screens.SettingsScreen
import com.fuwaki.synap.ui.screens.SearchScreen
import com.fuwaki.synap.ui.theme.MyApplicationTheme
import com.fuwaki.synap.ui.viewmodel.AppSessionUiState
import com.fuwaki.synap.ui.viewmodel.AppSessionViewModel
import com.fuwaki.synap.ui.viewmodel.DetailEvent
import com.fuwaki.synap.ui.viewmodel.DetailViewModel
import com.fuwaki.synap.ui.viewmodel.EditorEvent
import com.fuwaki.synap.ui.viewmodel.EditorMode
import com.fuwaki.synap.ui.viewmodel.EditorViewModel
import com.fuwaki.synap.ui.viewmodel.HomeViewModel
import com.fuwaki.synap.ui.viewmodel.SettingsViewModel
import dagger.hilt.android.AndroidEntryPoint
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.unit.sp
import javax.inject.Inject
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.foundation.layout.Arrangement

val LocalNoteTextSize = compositionLocalOf { 16.sp }

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    @Inject
    lateinit var synapService: SynapServiceApi

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            SynapApp()
        }
    }

    suspend fun exportDatabaseToUri(uri: Uri): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            contentResolver.openOutputStream(uri)?.use { output ->
                synapService.exportDatabase(output).getOrThrow()
            } ?: error("无法创建导出文件")
        }
    }

    suspend fun shareDatabase(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val cachePath = File(cacheDir, "exports")
            cachePath.mkdirs()
            val exportFile = File(cachePath, "synap_database.redb")

            exportFile.outputStream().use { output ->
                synapService.exportDatabase(output).getOrThrow()
            }

            val authority = "$packageName.fileprovider"
            val uri = FileProvider.getUriForFile(this@MainActivity, authority, exportFile)
            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = "application/octet-stream"
                putExtra(Intent.EXTRA_STREAM, uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }

            withContext(Dispatchers.Main) {
                startActivity(Intent.createChooser(shareIntent, "分享数据库"))
            }
        }
    }

    suspend fun importDatabaseFromUri(uri: Uri): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            contentResolver.openInputStream(uri)?.use { input ->
                synapService.replaceDatabase(input).getOrThrow()
            } ?: error("无法读取导入文件")
        }
    }

    fun closeForDatabaseRestart() {
        finishAffinity()
    }
}

@Composable
private fun SynapApp() {
    val context = LocalContext.current
    val activity = context as? MainActivity
    val prefs = remember { context.getSharedPreferences("synap_settings", Context.MODE_PRIVATE) }
    val supportsMonet = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S

    var themeMode by remember { mutableIntStateOf(prefs.getInt("themeMode", 0)) }
    var useMonet by remember { mutableStateOf(prefs.getBoolean("useMonet", supportsMonet)) }
    var customThemeHue by remember { mutableFloatStateOf(prefs.getFloat("customThemeHue", 210f)) }

    var selectedLanguageIndex by remember { mutableIntStateOf(prefs.getInt("selectedLanguage", 0)) }
    val baseLanguages = remember { sampleLanguages() }
    val languages = remember(baseLanguages) { listOf("跟随系统语言设置") + baseLanguages }

    var noteTextSize by remember { mutableFloatStateOf(prefs.getFloat("noteTextSize", 16f)) }

    var handedness by remember { mutableStateOf(prefs.getString("handedness", "靠右") ?: "靠右") }

    val sessionViewModel: AppSessionViewModel = hiltViewModel()
    val sessionState by sessionViewModel.uiState.collectAsState()

    val isDarkTheme = when (themeMode) {
        1 -> false
        2 -> true
        else -> isSystemInDarkTheme()
    }

    CompositionLocalProvider(LocalNoteTextSize provides noteTextSize.sp) {
        MyApplicationTheme(
            darkTheme = isDarkTheme,
            dynamicColor = supportsMonet && useMonet,
        ) {
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
                    primary = customPrimary,
                    onPrimary = if (isDarkTheme) Color(0xFF202020) else Color.White,
                    primaryContainer = customPrimaryContainer,
                    onPrimaryContainer = customOnPrimaryContainer,
                    secondaryContainer = customPrimaryContainer,
                    onSecondaryContainer = customOnPrimaryContainer,
                    background = customBackground,
                    surface = customBackground,
                    surfaceVariant = customSurfaceVariant
                )
            } else {
                currentScheme
            }

            MaterialTheme(
                colorScheme = finalScheme,
                typography = MaterialTheme.typography,
                shapes = MaterialTheme.shapes
            ) {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background,
                ) {
                    when (val state = sessionState) {
                        AppSessionUiState.Initializing -> SessionLoadingScreen()
                        is AppSessionUiState.Error -> SessionErrorScreen(
                            message = state.message,
                            onRetry = sessionViewModel::initialize,
                        )
                        AppSessionUiState.Ready -> SynapNavGraph(
                            themeMode = themeMode,
                            onThemeModeChange = {
                                themeMode = it
                                prefs.edit().putInt("themeMode", it).apply()
                            },
                            useMonet = useMonet,
                            supportsMonet = supportsMonet,
                            onUseMonetChange = {
                                useMonet = it
                                prefs.edit().putBoolean("useMonet", it).apply()
                            },
                            customThemeHue = customThemeHue,
                            onCustomThemeHueChange = {
                                customThemeHue = it
                                prefs.edit().putFloat("customThemeHue", it).apply()
                            },
                            handedness = handedness,
                            onHandednessChange = {
                                handedness = it
                                prefs.edit().putString("handedness", it).apply()
                            },
                            languages = languages,
                            selectedLanguageIndex = selectedLanguageIndex,
                            onLanguageSelect = {
                                selectedLanguageIndex = it
                                prefs.edit().putInt("selectedLanguage", it).apply()
                            },
                            noteTextSize = noteTextSize,
                            onNoteTextSizeChange = {
                                noteTextSize = it
                                prefs.edit().putFloat("noteTextSize", it).apply()
                            },
                            databaseActivity = activity,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SynapNavGraph(
    themeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    handedness: String,
    onHandednessChange: (String) -> Unit,
    languages: List<String>,
    selectedLanguageIndex: Int,
    onLanguageSelect: (Int) -> Unit,
    noteTextSize: Float,
    onNoteTextSizeChange: (Float) -> Unit,
    databaseActivity: MainActivity?,
) {
    val navController = rememberNavController()
    val backStackEntry by navController.currentBackStackEntryAsState()

    val configuration = LocalConfiguration.current
    val isLargeScreen = configuration.screenWidthDp >= 600
    var showSettingsSidebar by rememberSaveable { mutableStateOf(false) }

    Box(modifier = Modifier.fillMaxSize()) {
        NavHost(
            navController = navController,
            startDestination = "home",
            enterTransition = {
                slideIntoContainer(AnimatedContentTransitionScope.SlideDirection.Left, animationSpec = tween(400))
            },
            exitTransition = {
                slideOutOfContainer(AnimatedContentTransitionScope.SlideDirection.Left, animationSpec = tween(400))
            },
            popEnterTransition = {
                slideIntoContainer(AnimatedContentTransitionScope.SlideDirection.Right, animationSpec = tween(400))
            },
            popExitTransition = {
                slideOutOfContainer(AnimatedContentTransitionScope.SlideDirection.Right, animationSpec = tween(400))
            },
            modifier = Modifier.fillMaxSize()
        ) {
            composable(
                route = "home",
                enterTransition = { fadeIn() },
                exitTransition = { fadeOut() },
                popEnterTransition = { fadeIn() },
                popExitTransition = { fadeOut() },
            ) {
                val viewModel: HomeViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                HomeScreen(
                    uiState = uiState,
                    onOpenSettings = {
                        if (isLargeScreen) {
                            showSettingsSidebar = true
                        } else {
                            navController.navigate("settings")
                        }
                    },
                    onComposeNote = { navController.navigate(editorRoute()) },
                    onOpenNote = { noteId -> navController.navigate(detailRoute(noteId)) },
                    onReplyToNote = { noteId, summary ->
                        navController.navigate(editorRoute(parentId = noteId, parentSummary = summary))
                    },
                    onToggleDeleted = viewModel::toggleDeleted,
                    onOpenSearch = { navController.navigate("search") },
                    onLoadMore = viewModel::loadMore,
                    onRefresh = viewModel::refresh,
                )
            }

            composable(
                route = "search",
                enterTransition = { slideIntoContainer(AnimatedContentTransitionScope.SlideDirection.Up, animationSpec = tween(300)) + fadeIn() },
                exitTransition = { slideOutOfContainer(AnimatedContentTransitionScope.SlideDirection.Down, animationSpec = tween(300)) + fadeOut() }
            ) {
                val viewModel: HomeViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                SearchScreen(
                    uiState = uiState,
                    onSearchQueryChange = viewModel::updateQuery,
                    onSubmitSearch = viewModel::submitSearch,
                    onClearSearch = viewModel::clearSearch,
                    onNavigateBack = { navController.popBackStack() },
                    onOpenNote = { noteId -> navController.navigate(detailRoute(noteId)) },
                    onToggleDeleted = viewModel::toggleDeleted
                )
            }

            composable(
                route = "detail/{noteId}",
                arguments = listOf(navArgument("noteId") { type = NavType.StringType }),
            ) {
                val viewModel: DetailViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                LaunchedEffect(viewModel) {
                    viewModel.events.collect { event ->
                        if (event is DetailEvent.NavigateBackAfterDelete) {
                            navController.popBackStack()
                        }
                    }
                }

                NoteDetailScreen(
                    uiState = uiState,
                    onNavigateBack = { navController.popBackStack() },
                    onNavigateHome = { navController.popBackStack("home", inclusive = false) },
                    onDelete = viewModel::deleteCurrentNote,
                    onReply = {
                        uiState.note?.let { note ->
                            navController.navigate(editorRoute(parentId = note.id, parentSummary = note.content))
                        }
                    },
                    onEdit = {
                        uiState.note?.let { note ->
                            navController.navigate(editorRoute(editNoteId = note.id))
                        }
                    },
                    onOpenRelatedNote = { noteId -> navController.navigate(detailRoute(noteId)) },
                    onLoadMoreReplies = viewModel::loadMoreReplies,
                    onRefresh = viewModel::refreshAll,
                )
            }

            composable("settings") {
                SettingsContainer(
                    themeMode = themeMode,
                    onThemeModeChange = onThemeModeChange,
                    useMonet = useMonet,
                    supportsMonet = supportsMonet,
                    onUseMonetChange = onUseMonetChange,
                    customThemeHue = customThemeHue,
                    onCustomThemeHueChange = onCustomThemeHueChange,
                    handedness = handedness,
                    onHandednessChange = onHandednessChange,
                    noteTextSize = noteTextSize,
                    onNoteTextSizeChange = onNoteTextSizeChange,
                    databaseActivity = databaseActivity,
                    onNavigateToLanguageSelection = { navController.navigate("language_selection") },
                    onNavigateBack = { navController.popBackStack() }
                )
            }

            composable("language_selection") {
                LanguageSelectionScreen(
                    languages = languages,
                    selectedIndex = selectedLanguageIndex,
                    onLanguageSelect = onLanguageSelect,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(
                route = "editor?parentId={parentId}&parentSummary={parentSummary}&editNoteId={editNoteId}",
                arguments = listOf(
                    navArgument("parentId") { nullable = true; defaultValue = null; type = NavType.StringType },
                    navArgument("parentSummary") { nullable = true; defaultValue = null; type = NavType.StringType },
                    navArgument("editNoteId") { nullable = true; defaultValue = null; type = NavType.StringType },
                ),
            ) {
                val viewModel: EditorViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                LaunchedEffect(viewModel, backStackEntry?.destination?.route) {
                    viewModel.events.collect { event ->
                        if (event is EditorEvent.Saved) {
                            if (event.mode is EditorMode.Edit) {
                                navController.popBackStack()
                                navController.popBackStack()
                            } else {
                                navController.popBackStack()
                            }
                            navController.navigate(detailRoute(event.noteId))
                        }
                    }
                }

                NewNoteScreen(
                    uiState = uiState,
                    onNavigateBack = { navController.popBackStack() },
                    onContentChange = viewModel::updateContent,
                    onAddTag = viewModel::addTag,
                    onUpdateTag = viewModel::updateTag,
                    onRemoveTag = viewModel::removeTag,
                    onSave = viewModel::save,
                )
            }
        }

        AnimatedVisibility(
            visible = isLargeScreen && showSettingsSidebar,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier.fillMaxSize()
        ) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.4f))
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                        onClick = { showSettingsSidebar = false }
                    )
            )
        }

        AnimatedVisibility(
            visible = isLargeScreen && showSettingsSidebar,
            enter = slideInHorizontally(initialOffsetX = { it }),
            exit = slideOutHorizontally(targetOffsetX = { it }),
            modifier = Modifier.align(Alignment.CenterEnd)
        ) {
            Surface(
                modifier = Modifier
                    .width(320.dp)
                    .fillMaxHeight(),
                shape = RoundedCornerShape(topStart = 16.dp, bottomStart = 16.dp),
                shadowElevation = 8.dp,
                color = MaterialTheme.colorScheme.surface
            ) {
                SettingsContainer(
                    themeMode = themeMode,
                    onThemeModeChange = onThemeModeChange,
                    useMonet = useMonet,
                    supportsMonet = supportsMonet,
                    onUseMonetChange = onUseMonetChange,
                    customThemeHue = customThemeHue,
                    onCustomThemeHueChange = onCustomThemeHueChange,
                    handedness = handedness,
                    onHandednessChange = onHandednessChange,
                    noteTextSize = noteTextSize,
                    onNoteTextSizeChange = onNoteTextSizeChange,
                    databaseActivity = databaseActivity,
                    onNavigateToLanguageSelection = {
                        showSettingsSidebar = false
                        navController.navigate("language_selection")
                    },
                    onNavigateBack = { showSettingsSidebar = false }
                )
            }
        }
    }
}

@Composable
private fun SettingsContainer(
    themeMode: Int,
    onThemeModeChange: (Int) -> Unit,
    useMonet: Boolean,
    supportsMonet: Boolean,
    onUseMonetChange: (Boolean) -> Unit,
    customThemeHue: Float,
    onCustomThemeHueChange: (Float) -> Unit,
    handedness: String,
    onHandednessChange: (String) -> Unit,
    noteTextSize: Float,
    onNoteTextSizeChange: (Float) -> Unit,
    databaseActivity: MainActivity?,
    onNavigateToLanguageSelection: () -> Unit,
    onNavigateBack: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val settingsViewModel: SettingsViewModel = hiltViewModel()
    val settingsUiState by settingsViewModel.uiState.collectAsState()

    var showImportWarning by remember { mutableStateOf(false) }
    var showRestartRequired by remember { mutableStateOf(false) }
    // --- 新增：文件类型错误弹窗状态 ---
    var showFileTypeError by remember { mutableStateOf(false) }

    val exportDatabaseLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("application/octet-stream"),
    ) { uri ->
        if (uri == null || databaseActivity == null) {
            return@rememberLauncherForActivityResult
        }

        scope.launch {
            databaseActivity.exportDatabaseToUri(uri)
                .onSuccess {
                    Toast.makeText(context, "数据库已导出", Toast.LENGTH_SHORT).show()
                }
                .onFailure { throwable ->
                    Toast.makeText(
                        context,
                        throwable.message ?: "导出数据库失败",
                        Toast.LENGTH_LONG,
                    ).show()
                }
        }
    }

    val importDatabaseLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null || databaseActivity == null) {
            return@rememberLauncherForActivityResult
        }

        // --- 核心修改：检查文件后缀名 ---
        var isRedbFile = false
        context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) {
                val displayNameIndex = cursor.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                if (displayNameIndex != -1) {
                    val displayName = cursor.getString(displayNameIndex)
                    if (displayName != null && displayName.endsWith(".redb", ignoreCase = true)) {
                        isRedbFile = true
                    }
                }
            }
        }

        if (!isRedbFile) {
            // 拦截：显示文件类型错误弹窗
            showFileTypeError = true
        } else {
            // 通过校验：继续原有的导入逻辑
            scope.launch {
                databaseActivity.importDatabaseFromUri(uri)
                    .onSuccess {
                        showRestartRequired = true
                    }
                    .onFailure { throwable ->
                        Toast.makeText(
                            context,
                            throwable.message ?: "导入数据库失败",
                            Toast.LENGTH_LONG,
                        ).show()
                    }
            }
        }
    }

    if (showImportWarning) {
        AlertDialog(
            onDismissRequest = { showImportWarning = false },
            title = { Text("导入备份并替换") },
            text = {
                Text("注意！导入备份文件后，原来的数据将会被清空，请谨慎操作。导入后需要重启 App 生效")
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showImportWarning = false
                        importDatabaseLauncher.launch(arrayOf("*/*"))
                    },
                ) {
                    Text("选择数据库")
                }
            },
            dismissButton = {
                TextButton(onClick = { showImportWarning = false }) {
                    Text("取消")
                }
            },
        )
    }

    // --- 新增：文件类型错误的 UI 弹窗 ---
    if (showFileTypeError) {
        AlertDialog(
            onDismissRequest = { showFileTypeError = false },
            title = { Text("文件类型错误") },
            text = {
                Text("请上传“.redb”格式的备份文件")
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showFileTypeError = false
                        importDatabaseLauncher.launch(arrayOf("*/*")) // 重新唤起文件选择器
                    },
                ) {
                    Text("重新选择")
                }
            },
            dismissButton = {
                TextButton(onClick = { showFileTypeError = false }) {
                    Text("取消")
                }
            },
        )
    }

    if (showRestartRequired) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text("需要重启应用") },
            text = {
                Text("已成功导入备份数据，重启应用后生效。")
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showRestartRequired = false
                        databaseActivity?.closeForDatabaseRestart()
                    },
                ) {
                    Text("关闭应用")
                }
            },
        )
    }

    SettingsScreen(
        currentThemeMode = themeMode,
        onThemeModeChange = onThemeModeChange,
        useMonet = useMonet,
        supportsMonet = supportsMonet,
        onUseMonetChange = onUseMonetChange,
        customThemeHue = customThemeHue,
        onCustomThemeHueChange = onCustomThemeHueChange,
        handedness = handedness,
        onHandednessChange = onHandednessChange,
        noteTextSize = noteTextSize,
        onNoteTextSizeChange = onNoteTextSizeChange,
        buildVersion = settingsUiState.buildVersion,
        buildVersionDetails = settingsUiState.buildVersionDetails,
        onExportNotes = {
            scope.launch(Dispatchers.IO) {
                val testJsonData = "[\n  {\n    \"id\": \"1\",\n    \"content\": \"这是一条导出测试笔记。\"\n  }\n]"
                withContext(Dispatchers.Main) {
                    exportDataToZipAndShare(context, testJsonData)
                }
            }
        },
        onExportDatabase = {
            exportDatabaseLauncher.launch("synap_database.redb")
        },
        onShareDatabase = {
            if (databaseActivity == null) {
                Toast.makeText(context, "当前无法分享数据库", Toast.LENGTH_SHORT).show()
            } else {
                scope.launch {
                    databaseActivity.shareDatabase()
                        .onFailure { throwable ->
                            Toast.makeText(
                                context,
                                throwable.message ?: "分享数据库失败",
                                Toast.LENGTH_LONG,
                            ).show()
                        }
                }
            }
        },
        onImportDatabase = {
            if (databaseActivity == null) {
                Toast.makeText(context, "当前无法导入数据库", Toast.LENGTH_SHORT).show()
            } else {
                showImportWarning = true
            }
        },
        onNavigateToLanguageSelection = onNavigateToLanguageSelection,
        onNavigateBack = onNavigateBack,
    )
}

private fun detailRoute(noteId: String): String = "detail/${Uri.encode(noteId)}"

private fun editorRoute(parentId: String? = null, parentSummary: String? = null, editNoteId: String? = null): String {
    val params = buildList {
        parentId?.let { add("parentId=${Uri.encode(it)}") }
        parentSummary?.let { add("parentSummary=${Uri.encode(it.take(120))}") }
        editNoteId?.let { add("editNoteId=${Uri.encode(it)}") }
    }
    return if (params.isEmpty()) "editor" else "editor?${params.joinToString("&")}"
}

@Composable
private fun SessionLoadingScreen() {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        CircularProgressIndicator()
        Text("正在初始化 Synap...", modifier = Modifier.padding(top = 16.dp), style = MaterialTheme.typography.bodyLarge)
    }
}

@Composable
private fun SessionErrorScreen(message: String, onRetry: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Text("启动失败", style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.error)
        Text(message, modifier = Modifier.padding(top = 12.dp), style = MaterialTheme.typography.bodyMedium)
        Button(onClick = onRetry, modifier = Modifier.padding(top = 20.dp)) { Text("重试") }
    }
}

fun exportDataToZipAndShare(context: Context, jsonData: String) {
    try {
        val cachePath = File(context.cacheDir, "exports")
        cachePath.mkdirs()
        val zipFile = File(cachePath, "synap_backup.zip")

        ZipOutputStream(FileOutputStream(zipFile)).use { zos ->
            val entry = ZipEntry("synap_notes.json")
            zos.putNextEntry(entry)
            zos.write(jsonData.toByteArray(Charsets.UTF_8))
            zos.closeEntry()
        }

        val authority = "${context.packageName}.fileprovider"
        val uri: Uri = FileProvider.getUriForFile(context, authority, zipFile)

        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "application/zip"
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(shareIntent, "导出备份文件"))
    } catch (e: Exception) {
        e.printStackTrace()
    }
}