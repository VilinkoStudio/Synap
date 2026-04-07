package com.synap.app.ui.navigation

import android.net.Uri
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable

import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.synap.app.MainActivity
import com.synap.app.ui.screens.*
import com.synap.app.ui.viewmodel.*

fun detailRoute(noteId: String): String = "detail/${Uri.encode(noteId)}"

fun editorRoute(parentId: String? = null, parentSummary: String? = null, editNoteId: String? = null): String {
    val params = buildList {
        parentId?.let { add("parentId=${Uri.encode(it)}") }
        parentSummary?.let { add("parentSummary=${Uri.encode(it.take(120))}") }
        editNoteId?.let { add("editNoteId=${Uri.encode(it)}") }
    }
    return if (params.isEmpty()) "editor" else "editor?${params.joinToString("&")}"
}

@Composable
fun SynapNavGraph(
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
    currentFontFamily: String,
    onFontFamilyChange: (String) -> Unit,
    currentFontWeight: Int, // --- 新增参数 ---
    onFontWeightChange: (Int) -> Unit, // --- 新增参数 ---
    noteTextSize: Float,
    onNoteTextSizeChange: (Float) -> Unit,
    hasSeenTutorial: Boolean,
    onTutorialFinished: () -> Unit,
    databaseActivity: MainActivity?,
) {
    val navController = rememberNavController()

    val startDestination = remember { if (hasSeenTutorial) "home" else "tutorial" }

    Box(modifier = Modifier.fillMaxSize()) {
        NavHost(
            navController = navController,
            startDestination = startDestination,
            enterTransition = { slideIntoContainer(AnimatedContentTransitionScope.SlideDirection.Left, tween(400)) },
            exitTransition = { slideOutOfContainer(AnimatedContentTransitionScope.SlideDirection.Left, tween(400)) },
            popEnterTransition = { slideIntoContainer(AnimatedContentTransitionScope.SlideDirection.Right, tween(400)) },
            popExitTransition = { slideOutOfContainer(AnimatedContentTransitionScope.SlideDirection.Right, tween(400)) },
            modifier = Modifier.fillMaxSize()
        ) {
            composable("tutorial") {
                TutorialScreen(
                    onFinishTutorial = {
                        onTutorialFinished()
                        navController.navigate("home") { popUpTo("tutorial") { inclusive = true } }
                    }
                )
            }

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
                    onReplyToNote = { noteId, summary -> navController.navigate(editorRoute(parentId = noteId, parentSummary = summary)) },
                    onToggleDeleted = viewModel::toggleDeleted,
                    onOpenSearch = { navController.navigate("search") },
                    onOpenTrash = { navController.navigate("trash") },
                    onLoadMore = viewModel::loadMore,
                    onRefresh = viewModel::refresh,
                    onSetFilterPanelOpen = viewModel::setFilterPanelOpen,
                    onToggleTagFilter = viewModel::toggleTag,
                    onToggleUntaggedFilter = viewModel::toggleUntagged,
                    onToggleAllTags = viewModel::toggleAllTags,
                )
            }

            composable("search") {
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

            composable("trash") {
                val viewModel: TrashViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                TrashScreen(
                    uiState = uiState,
                    onNavigateBack = { navController.popBackStack() },
                    onRestoreNote = viewModel::restoreNote,
                    onLoadMore = viewModel::loadMore,
                    onRefresh = viewModel::refresh,
                )
            }

            composable("detail/{noteId}", arguments = listOf(navArgument("noteId") { type = NavType.StringType })) {
                val viewModel: DetailViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                LaunchedEffect(viewModel) {
                    viewModel.events.collect { if (it is DetailEvent.NavigateBackAfterDelete) navController.popBackStack() }
                }

                NoteDetailScreen(
                    uiState = uiState,
                    onNavigateBack = { navController.popBackStack() },
                    onNavigateHome = { navController.popBackStack("home", inclusive = false) },
                    onDelete = viewModel::deleteCurrentNote,
                    onReply = { uiState.note?.let { note -> navController.navigate(editorRoute(parentId = note.id, parentSummary = note.content)) } },
                    onEdit = { uiState.note?.let { note -> navController.navigate(editorRoute(editNoteId = note.id)) } },
                    onOpenRelatedNote = { noteId -> navController.navigate(detailRoute(noteId)) },
                    onLoadMoreReplies = viewModel::loadMoreReplies,
                    onRefresh = viewModel::refreshAll,
                )
            }

            composable("settings") {
                SettingsContainer(
                    themeMode = themeMode, onThemeModeChange = onThemeModeChange,
                    useMonet = useMonet, supportsMonet = supportsMonet, onUseMonetChange = onUseMonetChange,
                    customThemeHue = customThemeHue, onCustomThemeHueChange = onCustomThemeHueChange,
                    handedness = handedness, onHandednessChange = onHandednessChange,
                    databaseActivity = databaseActivity,
                    onNavigateToTypographySettings = { navController.navigate("typography_settings") },
                    onNavigateToLanguageSelection = { navController.navigate("language_selection") },
                    onNavigateToTeam = { navController.navigate("team") },
                    onNavigateToTutorial = { navController.navigate("tutorial") },
                    onNavigateBack = { navController.popBackStack() }
                )
            }

            composable("language_selection") {
                LanguageSelectionContainer(
                    languages = languages, selectedIndex = selectedLanguageIndex,
                    onLanguageSelect = onLanguageSelect, onNavigateBack = { navController.popBackStack() }
                )
            }

            composable("typography_settings") {
                TypographySettingsContainer(
                    currentFontFamily = currentFontFamily, onFontFamilyChange = onFontFamilyChange,
                    currentFontWeight = currentFontWeight, onFontWeightChange = onFontWeightChange,
                    noteTextSize = noteTextSize, onNoteTextSizeChange = onNoteTextSizeChange,
                    onNavigateBack = { navController.popBackStack() }
                )
            }

            composable("team") {
                TeamScreen(
                    onNavigateBack = { navController.popBackStack() }
                )
            }

            composable(
                route = "editor?parentId={parentId}&parentSummary={parentSummary}&editNoteId={editNoteId}",
                arguments = listOf(
                    navArgument("parentId") { nullable = true; type = NavType.StringType },
                    navArgument("parentSummary") { nullable = true; type = NavType.StringType },
                    navArgument("editNoteId") { nullable = true; type = NavType.StringType },
                ),
            ) {
                val viewModel: EditorViewModel = hiltViewModel()
                val uiState by viewModel.uiState.collectAsState()

                LaunchedEffect(viewModel) {
                    viewModel.events.collect { event ->
                        if (event is EditorEvent.Saved) {
                            if (event.mode is EditorMode.Edit) {
                                navController.popBackStack()
                                navController.popBackStack()
                            } else navController.popBackStack()
                            navController.navigate(detailRoute(event.noteId))
                        }
                    }
                }

                NewNoteScreen(
                    uiState = uiState, onNavigateBack = { navController.popBackStack() },
                    onContentChange = viewModel::updateContent, onAddTag = viewModel::addTag,
                    onRemoveTag = viewModel::removeTag, onSave = viewModel::save,
                )
            }
        }

    }
}
