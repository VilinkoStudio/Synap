package com.synap.app.ui.navigation

import android.net.Uri
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionLayout
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import androidx.navigation.navDeepLink
import com.synap.app.MainActivity
import com.synap.app.ui.screens.*
import com.synap.app.ui.viewmodel.*

fun detailRoute(noteId: String): String = "detail/${Uri.encode(noteId)}"
fun threadReaderRoute(noteId: String): String = "thread/${Uri.encode(noteId)}"

// ========== 新增 initialContent 参数 ==========
fun editorRoute(parentId: String? = null, parentSummary: String? = null, editNoteId: String? = null, initialContent: String? = null): String {
    val params = buildList {
        parentId?.let { add("parentId=${Uri.encode(it)}") }
        parentSummary?.let { add("parentSummary=${Uri.encode(it.take(120))}") }
        editNoteId?.let { add("editNoteId=${Uri.encode(it)}") }
        initialContent?.let { add("initialContent=${Uri.encode(it)}") }
    }
    return if (params.isEmpty()) "editor" else "editor?${params.joinToString("&")}"
}

@OptIn(ExperimentalSharedTransitionApi::class)
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
    currentFontWeight: Int,
    onFontWeightChange: (Int) -> Unit,
    noteTextSize: Float,
    onNoteTextSizeChange: (Float) -> Unit,
    noteLineSpacing: Float,
    onNoteLineSpacingChange: (Float) -> Unit,
    hasSeenTutorial: Boolean,
    onTutorialFinished: () -> Unit,
    databaseActivity: MainActivity?,
    draftCapacity: Int,
    onDraftCapacityChange: (Int) -> Unit,
) {
    val navController = rememberNavController()
    val startDestination = remember { if (hasSeenTutorial) "home" else "tutorial" }

    SharedTransitionLayout {
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
                        currentThemeMode = themeMode,
                        onThemeModeChange = onThemeModeChange,
                        useMonet = useMonet,
                        supportsMonet = supportsMonet,
                        onUseMonetChange = onUseMonetChange,
                        customThemeHue = customThemeHue,
                        onCustomThemeHueChange = onCustomThemeHueChange,
                        availableLanguages = languages,
                        currentLanguageIndex = selectedLanguageIndex,
                        onLanguageSelect = onLanguageSelect,
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
                        onOpenStarmap = { navController.navigate("starmap") },
                        onOpenTrash = { navController.navigate("trash") },
                        onLoadMore = viewModel::loadMore,
                        onRefresh = viewModel::refresh,
                        onSetFilterPanelOpen = viewModel::setFilterPanelOpen,
                        onToggleTagFilter = viewModel::toggleTag,
                        onToggleUntaggedFilter = viewModel::toggleUntagged,
                        onToggleAllTags = viewModel::toggleAllTags,
                        onExportShare = viewModel::exportShare,
                        sharedTransitionScope = this@SharedTransitionLayout,
                        animatedVisibilityScope = this@composable
                    )
                }

                composable("starmap") {
                    val viewModel: StarmapViewModel = hiltViewModel()
                    val uiState by viewModel.uiState.collectAsState()

                    StarmapScreen(
                        uiState = uiState,
                        onNavigateBack = { navController.popBackStack() },
                        onRefresh = viewModel::refresh,
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
                        onOpenThreadReader = { noteId -> navController.navigate(threadReaderRoute(noteId)) },
                        onLoadMoreReplies = viewModel::loadMoreReplies,
                        onRefresh = viewModel::refreshAll,
                        onExportShare = viewModel::exportShare,
                    )
                }

                composable(
                    route = "thread/{noteId}",
                    arguments = listOf(
                        navArgument("noteId") { type = NavType.StringType },
                    ),
                ) {
                    val viewModel: ThreadReaderViewModel = hiltViewModel()
                    val uiState by viewModel.uiState.collectAsState()

                    ThreadReaderScreen(
                        uiState = uiState,
                        onNavigateBack = { navController.popBackStack() },
                        onOpenOriginDetail = { noteId -> navController.navigate(detailRoute(noteId)) },
                        onOpenBranch = viewModel::selectBranch,
                        onOpenNodeAsAnchor = viewModel::openNodeAsAnchor,
                        onShowBranchSheet = viewModel::openBranchSheet,
                        onDismissBranchSheet = viewModel::dismissBranchSheet,
                        onBacktrack = viewModel::goBackInHistory,
                        onRefresh = viewModel::refresh,
                        onOpenGraph = viewModel::openGraphSheet,
                        onDismissGraph = viewModel::dismissGraphSheet,
                        onFocusNode = viewModel::focusNode,
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
                        onNavigateToColorSettings = { navController.navigate("setting_color") },
                        onNavigateToLanguageSelection = { navController.navigate("language_selection") },
                        onNavigateToAppIcon = { navController.navigate("app_icon") },
                        onNavigateToHomeLayout = { navController.navigate("setting_home_layout") },
                        onNavigateToLab = { navController.navigate("lab") },
                        onNavigateToAIService = { navController.navigate("setting_ai_api") },
                        onNavigateToAIScenarios = { navController.navigate("setting_ai_scenarios") },
                        onNavigateToSync = { navController.navigate("sync") },
                        onNavigateToTeam = { navController.navigate("team") },
                        onNavigateToTutorial = { navController.navigate("tutorial") },
                        onNavigateBack = { navController.popBackStack() },
                        draftCapacity = draftCapacity,
                        onDraftCapacityChange = onDraftCapacityChange,
                    )
                }

                composable("sync") {
                    val viewModel: SyncViewModel = hiltViewModel()
                    val uiState by viewModel.uiState.collectAsState()

                    SyncScreen(
                        uiState = uiState,
                        onRefresh = viewModel::refresh,
                        onAddConnection = viewModel::addConnection,
                        onDeleteConnection = viewModel::deleteConnection,
                        onPairConnection = viewModel::pairConnection,
                        onPairDiscoveredPeer = viewModel::pairDiscoveredPeer,
                        onRelayBaseUrlChange = viewModel::updateRelayBaseUrl,
                        onRelayApiKeyChange = viewModel::updateRelayApiKey,
                        onSaveRelayConfig = viewModel::saveRelayConfig,
                        onRelayFetch = viewModel::fetchRelayUpdates,
                        onRelayPush = viewModel::pushRelayUpdates,
                        onTrustPeer = viewModel::trustPeer,
                        onUpdatePeerNote = viewModel::updatePeerNote,
                        onDeletePeer = viewModel::deletePeer,
                        onSetPeerStatus = viewModel::setPeerStatus,
                        onDismissPendingTrustPrompt = viewModel::dismissPendingTrustPrompt,
                        onNavigateBack = { navController.popBackStack() },
                    )
                }

                composable("app_icon") {
                    SettingLogoScreen(onNavigateBack = { navController.popBackStack() })
                }

                composable("setting_color") {
                    SettingColorScreen(
                        currentThemeMode = themeMode,
                        onThemeModeChange = onThemeModeChange,
                        useMonet = useMonet,
                        supportsMonet = supportsMonet,
                        onUseMonetChange = onUseMonetChange,
                        customThemeHue = customThemeHue,
                        onCustomThemeHueChange = onCustomThemeHueChange,
                        onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("setting_home_layout") {
                    val homeViewModel: HomeViewModel = hiltViewModel(navController.getBackStackEntry("home"))
                    SettingHomeScreen(
                        onSetFilterPanelOpen = homeViewModel::setFilterPanelOpen,
                        onRefresh = homeViewModel::refresh,
                        onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("setting_ai_api") {
                    SettingAIapiScreen(
                        onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("setting_ai_scenarios") {
                    SettingAIScenariosScreen(
                        onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("lab") {
                    LabSettingsScreen(
                        onNavigateToAIService = { navController.navigate("setting_ai_api") },
                        onNavigateToAIScenarios = { navController.navigate("setting_ai_scenarios") },
                        onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("language_selection") {
                    LanguageSelectionScreen(
                        languages = languages, selectedIndex = selectedLanguageIndex,
                        onLanguageSelect = onLanguageSelect, onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("typography_settings") {
                    TypographySettingsScreen(
                        currentFontFamily = currentFontFamily, onFontFamilyChange = onFontFamilyChange,
                        currentFontWeight = currentFontWeight, onFontWeightChange = onFontWeightChange,
                        noteTextSize = noteTextSize, onNoteTextSizeChange = onNoteTextSizeChange,
                        noteLineSpacing = noteLineSpacing, onNoteLineSpacingChange = onNoteLineSpacingChange,
                        onNavigateBack = { navController.popBackStack() }
                    )
                }

                composable("team") {
                    TeamScreen(onNavigateBack = { navController.popBackStack() })
                }

                composable(
                    route = "editor?parentId={parentId}&parentSummary={parentSummary}&editNoteId={editNoteId}&initialContent={initialContent}",
                    arguments = listOf(
                        navArgument("parentId") { nullable = true; type = NavType.StringType },
                        navArgument("parentSummary") { nullable = true; type = NavType.StringType },
                        navArgument("editNoteId") { nullable = true; type = NavType.StringType },
                        navArgument("initialContent") { nullable = true; type = NavType.StringType },
                    ),
                    deepLinks = listOf(
                        navDeepLink { uriPattern = "synap://editor?initialContent={initialContent}" },
                        navDeepLink { uriPattern = "synap://editor" }
                    )
                ) { backStackEntry ->
                    val viewModel: EditorViewModel = hiltViewModel()
                    val uiState by viewModel.uiState.collectAsState()
                    val context = LocalContext.current
                    val draftStore = remember { com.synap.app.data.service.DraftStore(context) }
                    var draftCount by remember { mutableIntStateOf(draftStore.count()) }

                    // 自动刷新草稿箱数量
                    LaunchedEffect(Unit) {
                        draftCount = draftStore.count()
                    }

                    // ========== 核心自动填充逻辑 ==========
                    val initialContent = backStackEntry.arguments?.getString("initialContent")
                    LaunchedEffect(initialContent) {
                        if (!initialContent.isNullOrBlank() && uiState.mode !is EditorMode.Edit && uiState.content.isBlank()) {
                            viewModel.updateContent(initialContent)
                        }
                    }

                    LaunchedEffect(viewModel) {
                        viewModel.events.collect { event ->
                            if (event is EditorEvent.Saved) {
                                draftCount = draftStore.count()
                                if (event.mode is EditorMode.Edit) {
                                    navController.popBackStack()
                                    navController.popBackStack()
                                } else navController.popBackStack()
                                navController.navigate(detailRoute(event.noteId))
                            }
                        }
                    }

                    NewNoteScreen(
                        uiState = uiState,
                        onNavigateBack = { navController.popBackStack() },
                        onNavigateToHome = {
                            navController.navigate("home") {
                                popUpTo("home") { inclusive = true }
                            }
                        },
                        onContentChange = viewModel::updateContent,
                        onAddTag = viewModel::addTag,
                        onRemoveTag = viewModel::removeTag,
                        onNoteColorHueChange = viewModel::setNoteColorHue,
                        onSave = viewModel::save,
                        onNavigateToDrafts = { navController.navigate("drafts") },
                        draftCount = draftCount,
                        hasUnsavedChanges = viewModel.hasUnsavedChanges(),
                        onSaveDraft = {
                            viewModel.saveDraftManually()
                            // 将当前草稿标记为已读
                            viewModel.getCurrentDraftId()?.let { viewModel.markDraftAsRead(it) }
                            draftCount = draftStore.count()
                        },
                        onDiscardDraft = {
                            // 删除当前草稿
                            viewModel.getCurrentDraftId()?.let { draftStore.delete(it) }
                            draftCount = draftStore.count()
                        },
                        isContentMatchingLatestDraft = viewModel::isContentMatchingLatestDraft,
                        onMarkDraftAsRead = viewModel::markDraftAsRead,
                        onRefreshDraftCount = { draftCount = draftStore.count() },
                        sharedTransitionScope = this@SharedTransitionLayout,
                        animatedVisibilityScope = this@composable
                    )
                }

                composable("drafts") {
                    val context = LocalContext.current
                    DraftScreen(
                        onNavigateBack = { navController.popBackStack() },
                        onDraftClick = { draft ->
                            // Navigate to editor with draft content
                            navController.navigate(
                                editorRoute(
                                    parentId = draft.parentId,
                                    parentSummary = draft.parentSummary,
                                    editNoteId = draft.editNoteId,
                                    initialContent = draft.content,
                                )
                            )
                            // Delete the draft after navigating to editor
                            val draftStore = com.synap.app.data.service.DraftStore(context)
                            draftStore.delete(draft.id)
                        },
                    )
                }
            }
        }
    }
}
