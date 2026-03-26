package com.fuwaki.synap

import android.net.Uri
import android.os.Bundle
import android.os.Build
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import androidx.hilt.navigation.compose.hiltViewModel
import com.fuwaki.synap.ui.data.sampleLanguages
import com.fuwaki.synap.ui.screens.HomeScreen
import com.fuwaki.synap.ui.screens.LanguageSelectionScreen
import com.fuwaki.synap.ui.screens.NewNoteScreen
import com.fuwaki.synap.ui.screens.NoteDetailScreen
import com.fuwaki.synap.ui.screens.SettingsScreen
import com.fuwaki.synap.ui.theme.MyApplicationTheme
import com.fuwaki.synap.ui.viewmodel.AppSessionUiState
import com.fuwaki.synap.ui.viewmodel.AppSessionViewModel
import com.fuwaki.synap.ui.viewmodel.DetailEvent
import com.fuwaki.synap.ui.viewmodel.DetailViewModel
import com.fuwaki.synap.ui.viewmodel.EditorEvent
import com.fuwaki.synap.ui.viewmodel.EditorMode
import com.fuwaki.synap.ui.viewmodel.EditorViewModel
import com.fuwaki.synap.ui.viewmodel.HomeViewModel
import dagger.hilt.android.AndroidEntryPoint
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            SynapApp()
        }
    }
}

@Composable
private fun SynapApp() {
    var themeMode by rememberSaveable { mutableIntStateOf(0) }
    val supportsMonet = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
    var useMonet by rememberSaveable { mutableStateOf(supportsMonet) }
    var isSystemLanguage by rememberSaveable { mutableStateOf(true) }
    var selectedLanguageIndex by rememberSaveable { mutableIntStateOf(0) }
    val sessionViewModel: AppSessionViewModel = hiltViewModel()
    val sessionState by sessionViewModel.uiState.collectAsState()
    val isDarkTheme = when (themeMode) {
        1 -> false
        2 -> true
        else -> isSystemInDarkTheme()
    }

    val languages = remember { sampleLanguages() }

    MyApplicationTheme(
        darkTheme = isDarkTheme,
        dynamicColor = supportsMonet && useMonet,
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
                    onThemeModeChange = { themeMode = it },
                    useMonet = useMonet,
                    supportsMonet = supportsMonet,
                    onUseMonetChange = { useMonet = it },
                    isSystemLanguage = isSystemLanguage,
                    onSystemLanguageToggle = { isSystemLanguage = it },
                    languages = languages,
                    selectedLanguageIndex = selectedLanguageIndex,
                    onLanguageSelect = { selectedLanguageIndex = it },
                )
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
    isSystemLanguage: Boolean,
    onSystemLanguageToggle: (Boolean) -> Unit,
    languages: List<String>,
    selectedLanguageIndex: Int,
    onLanguageSelect: (Int) -> Unit,
) {
    val navController = rememberNavController()
    val backStackEntry by navController.currentBackStackEntryAsState()

    NavHost(
        navController = navController,
        startDestination = "home",
        enterTransition = {
            slideIntoContainer(
                AnimatedContentTransitionScope.SlideDirection.Left,
                animationSpec = tween(400),
            )
        },
        exitTransition = {
            slideOutOfContainer(
                AnimatedContentTransitionScope.SlideDirection.Left,
                animationSpec = tween(400),
            )
        },
        popEnterTransition = {
            slideIntoContainer(
                AnimatedContentTransitionScope.SlideDirection.Right,
                animationSpec = tween(400),
            )
        },
        popExitTransition = {
            slideOutOfContainer(
                AnimatedContentTransitionScope.SlideDirection.Right,
                animationSpec = tween(400),
            )
        },
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
                onOpenSettings = { navController.navigate("settings") },
                onComposeNote = { navController.navigate(editorRoute()) },
                onOpenNote = { noteId -> navController.navigate(detailRoute(noteId)) },
                onReplyToNote = { noteId, summary ->
                    navController.navigate(editorRoute(parentId = noteId, parentSummary = summary))
                },
                onToggleDeleted = viewModel::toggleDeleted,
                onToggleDeletedFeed = viewModel::toggleDeletedFeed,
                onSearchQueryChange = viewModel::updateQuery,
                onSubmitSearch = viewModel::submitSearch,
                onClearSearch = viewModel::clearSearch,
                onLoadMore = viewModel::loadMore,
                onRefresh = viewModel::refresh,
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
                onOpenRelatedNote = { noteId ->
                    navController.navigate(detailRoute(noteId))
                },
                onLoadMoreReplies = viewModel::loadMoreReplies,
                onRefresh = viewModel::refreshAll,
            )
        }

        composable("settings") {
            SettingsScreen(
                currentThemeMode = themeMode,
                onThemeModeChange = onThemeModeChange,
                useMonet = useMonet,
                supportsMonet = supportsMonet,
                onUseMonetChange = onUseMonetChange,
                isSystemLanguage = isSystemLanguage,
                onSystemLanguageToggle = onSystemLanguageToggle,
                onNavigateToLanguageSelection = { navController.navigate("language_selection") },
                onNavigateBack = { navController.popBackStack() },
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
                navArgument("parentId") {
                    nullable = true
                    defaultValue = null
                    type = NavType.StringType
                },
                navArgument("parentSummary") {
                    nullable = true
                    defaultValue = null
                    type = NavType.StringType
                },
                navArgument("editNoteId") {
                    nullable = true
                    defaultValue = null
                    type = NavType.StringType
                },
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
}

private fun detailRoute(noteId: String): String = "detail/${Uri.encode(noteId)}"

private fun editorRoute(
    parentId: String? = null,
    parentSummary: String? = null,
    editNoteId: String? = null,
): String {
    val params = buildList {
        parentId?.let { add("parentId=${Uri.encode(it)}") }
        parentSummary?.let { add("parentSummary=${Uri.encode(it.take(120))}") }
        editNoteId?.let { add("editNoteId=${Uri.encode(it)}") }
    }

    return if (params.isEmpty()) {
        "editor"
    } else {
        "editor?${params.joinToString("&")}"
    }
}

@Composable
private fun SessionLoadingScreen() {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        CircularProgressIndicator()
        Text(
            text = "正在初始化 Synap...",
            modifier = Modifier.padding(top = 16.dp),
            style = MaterialTheme.typography.bodyLarge,
        )
    }
}

@Composable
private fun SessionErrorScreen(message: String, onRetry: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = "启动失败",
            style = MaterialTheme.typography.headlineSmall,
            color = MaterialTheme.colorScheme.error,
        )
        Text(
            text = message,
            modifier = Modifier.padding(top = 12.dp),
            style = MaterialTheme.typography.bodyMedium,
        )
        Button(
            onClick = onRetry,
            modifier = Modifier.padding(top = 20.dp),
        ) {
            Text("重试")
        }
    }
}
