package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.NoteFeedStatus
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.portal.CursorPortal
import com.synap.app.data.portal.PortalState
import com.synap.app.data.repository.SynapRepository
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.TimelineSessionGroup
import com.synap.app.ui.model.toUiNote
import com.synap.app.ui.model.toUiSessionGroup
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class HomeUiState(
    val query: String = "",
    val notes: List<Note> = emptyList(),
    val sessionGroups: List<TimelineSessionGroup> = emptyList(),
    val isLoading: Boolean = true,
    val hasMore: Boolean = false,
    val isSearchMode: Boolean = false,
    val isFilterPanelOpen: Boolean = false,
    val showSessionFeed: Boolean = true,
    val availableTags: List<String> = emptyList(),
    val unselectedTags: Set<String> = emptySet(),
    val isUntaggedUnselected: Boolean = false,
    val errorMessage: String? = null,
)

private data class HomeQueryState(
    val query: String,
    val searchResults: List<Note>,
    val isSearchLoading: Boolean,
    val searchError: String?,
)

private data class HomeFilterState(
    val availableTags: List<String>,
    val unselectedTags: Set<String>,
    val isUntaggedUnselected: Boolean,
    val isFilterPanelOpen: Boolean,
) {
    fun toFeedFilter(): NoteFeedFilter {
        val selectedTags = availableTags.filterNot { it in unselectedTags }
        return NoteFeedFilter(
            selectedTags = selectedTags,
            includeUntagged = !isUntaggedUnselected,
            tagFilterEnabled = isFilterPanelOpen && (unselectedTags.isNotEmpty() || isUntaggedUnselected),
            status = NoteFeedStatus.Normal,
        )
    }

    // 核心逻辑：只要筛选面板打开，就显示瀑布流；关闭则显示时间组
    fun shouldShowSessionFeed(): Boolean = !isFilterPanelOpen
}

private data class HomeFeedState(
    val notes: List<Note>,
    val sessionGroups: List<TimelineSessionGroup>,
    val isLoading: Boolean,
    val hasMore: Boolean,
    val isFilterPanelOpen: Boolean,
    val showSessionFeed: Boolean,
    val availableTags: List<String>,
    val unselectedTags: Set<String>,
    val isUntaggedUnselected: Boolean,
    val errorMessage: String?,
)

@HiltViewModel
class HomeViewModel @Inject constructor(
    private val repository: SynapRepository,
) : ViewModel() {
    private val pageLimit = 20u
    private val recentPortal = repository.openRecentPortal(limit = pageLimit)
    private val recentSessionsPortal = repository.openRecentSessionsPortal(limit = pageLimit)
    private val query = MutableStateFlow("")
    private val searchResults = MutableStateFlow<List<Note>>(emptyList())
    private val isSearchLoading = MutableStateFlow(false)
    private val searchError = MutableStateFlow<String?>(null)
    private val feedError = MutableStateFlow<String?>(null)
    private val availableTags = MutableStateFlow<List<String>>(emptyList())
    private val unselectedTags = MutableStateFlow<Set<String>>(emptySet())
    private val isUntaggedUnselected = MutableStateFlow(false)
    private val isFilterPanelOpen = MutableStateFlow(false)
    private val filteredPortalState = MutableStateFlow(PortalState<NoteRecord>())
    private var filteredPortal: CursorPortal<NoteRecord>? = null
    private var filteredPortalKey: NoteFeedFilter? = null

    private val queryState = combine(
        query,
        searchResults,
        isSearchLoading,
        searchError,
    ) { currentQuery, currentSearchResults, searching, currentSearchError ->
        HomeQueryState(
            query = currentQuery,
            searchResults = currentSearchResults,
            isSearchLoading = searching,
            searchError = currentSearchError,
        )
    }

    private val filterState = combine(
        availableTags,
        unselectedTags,
        isUntaggedUnselected,
        isFilterPanelOpen,
    ) { currentTags, currentUnselectedTags, currentIsUntaggedUnselected, currentIsFilterPanelOpen ->
        HomeFilterState(
            availableTags = currentTags,
            unselectedTags = currentUnselectedTags.intersect(currentTags.toSet()),
            isUntaggedUnselected = currentIsUntaggedUnselected,
            isFilterPanelOpen = currentIsFilterPanelOpen,
        )
    }

    private val homeFeedState = combine(
        recentPortal.state,
        recentSessionsPortal.state,
        filteredPortalState,
        filterState,
    ) { recent, recentSessions, filtered, currentFilterState ->
        val feedFilter = currentFilterState.toFeedFilter()
        val useFilteredPortal = shouldUseFilteredPortal(feedFilter)
        val showSessionFeed = currentFilterState.shouldShowSessionFeed()

        val noteFeed = if (useFilteredPortal) filtered else recent
        val homeNotes = noteFeed.items.map { record -> record.toUiNote() }
        val sessionGroups = recentSessions.items.map { session -> session.toUiSessionGroup() }

        HomeFeedState(
            notes = homeNotes,
            sessionGroups = sessionGroups,
            isLoading = if (showSessionFeed) recentSessions.isLoading else noteFeed.isLoading,
            hasMore = if (showSessionFeed) recentSessions.hasMore else noteFeed.hasMore,
            isFilterPanelOpen = currentFilterState.isFilterPanelOpen,
            showSessionFeed = showSessionFeed,
            availableTags = currentFilterState.availableTags,
            unselectedTags = currentFilterState.unselectedTags,
            isUntaggedUnselected = currentFilterState.isUntaggedUnselected,
            errorMessage = if (showSessionFeed) {
                recentSessions.error?.message
            } else {
                noteFeed.error?.message
            },
        )
    }

    val uiState: StateFlow<HomeUiState> = combine(
        homeFeedState,
        queryState,
        feedError,
    ) { currentHomeFeed, currentState, currentFeedError ->
        val searchMode = currentState.query.isNotBlank()

        HomeUiState(
            query = currentState.query,
            notes = if (searchMode) currentState.searchResults else currentHomeFeed.notes,
            sessionGroups = if (searchMode) emptyList() else currentHomeFeed.sessionGroups,
            isLoading = if (searchMode) currentState.isSearchLoading else currentHomeFeed.isLoading,
            hasMore = if (searchMode) false else currentHomeFeed.hasMore,
            isSearchMode = searchMode,
            isFilterPanelOpen = currentHomeFeed.isFilterPanelOpen,
            showSessionFeed = !searchMode && currentHomeFeed.showSessionFeed,
            availableTags = currentHomeFeed.availableTags,
            unselectedTags = currentHomeFeed.unselectedTags,
            isUntaggedUnselected = currentHomeFeed.isUntaggedUnselected,
            errorMessage = if (searchMode) {
                currentState.searchError
            } else {
                currentHomeFeed.errorMessage ?: currentFeedError
            },
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = HomeUiState(),
    )

    init {
        refresh()

        viewModelScope.launch {
            repository.mutations.collect {
                invalidateHomePortals()
                refreshAvailableTags()
                if (query.value.isBlank()) {
                    refreshHomeFeed()
                } else {
                    rerunSearch()
                }
            }
        }
    }

    fun updateQuery(value: String) {
        query.value = value
        if (value.isBlank()) {
            searchResults.value = emptyList()
            isSearchLoading.value = false
            searchError.value = null
        }
    }

    fun submitSearch() {
        viewModelScope.launch {
            rerunSearch()
        }
    }

    fun clearSearch() {
        updateQuery("")
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            refreshAvailableTags()
            refreshHomeFeed()
        }
    }

    fun loadMore() {
        if (query.value.isNotBlank()) {
            return
        }

        viewModelScope.launch {
            loadMoreHomeFeed()
        }
    }

    suspend fun exportShare(noteIds: List<String>): ByteArray =
        repository.exportShare(noteIds)

    fun toggleDeleted(note: Note) {
        viewModelScope.launch {
            runCatching {
                if (note.isDeleted) {
                    repository.restoreNote(note.id)
                } else {
                    repository.deleteNote(note.id)
                }
            }.onFailure { throwable ->
                feedError.value = throwable.message ?: "Unable to update note"
            }
        }
    }

    // ==================== 核心修复：完美支持“多选”的正选逻辑 ====================

    fun toggleTag(tag: String) {
        val allTags = availableTags.value.toSet()
        val isAllSelected = unselectedTags.value.isEmpty() && !isUntaggedUnselected.value

        if (isAllSelected) {
            // 当前是“全部”状态，点击某个标签后，变成“仅选中该标签”
            // 做法：将其他所有标签放入排除列表
            unselectedTags.value = allTags - tag
            isUntaggedUnselected.value = true
        } else {
            // 当前已经是多选状态，反转这个标签的选中状态
            val currentUnselected = unselectedTags.value.toMutableSet()
            if (tag in currentUnselected) {
                // 如果之前没被选中(在排除列表里)，现在选中它
                currentUnselected.remove(tag)
            } else {
                // 如果之前被选中了(不在排除列表里)，现在取消选中它
                currentUnselected.add(tag)
            }

            // 触底判断：如果取消选中后，所有的标签和“未分类”都处于未选中状态，则恢复“全部”亮起
            if (currentUnselected.size == allTags.size && isUntaggedUnselected.value) {
                unselectedTags.value = emptySet()
                isUntaggedUnselected.value = false
            } else {
                unselectedTags.value = currentUnselected
            }
        }
        triggerFilterRefresh()
    }

    fun toggleUntagged() {
        val allTags = availableTags.value.toSet()
        val isAllSelected = unselectedTags.value.isEmpty() && !isUntaggedUnselected.value

        if (isAllSelected) {
            // 当前是“全部”状态，点击未分类后，变成“仅选中未分类”
            unselectedTags.value = allTags
            isUntaggedUnselected.value = false
        } else {
            // 当前已经是多选状态，反转未分类的选中状态
            val newUntaggedUnselected = !isUntaggedUnselected.value

            // 触底判断：如果取消选中后，什么都没选中了，则恢复“全部”亮起
            if (unselectedTags.value.size == allTags.size && newUntaggedUnselected) {
                unselectedTags.value = emptySet()
                isUntaggedUnselected.value = false
            } else {
                isUntaggedUnselected.value = newUntaggedUnselected
            }
        }
        triggerFilterRefresh()
    }

    fun toggleAllTags() {
        // 点击“全部”，意味着清空所有过滤排除条件，显示全部
        unselectedTags.value = emptySet()
        isUntaggedUnselected.value = false
        triggerFilterRefresh()
    }

    // ================================================================

    fun setFilterPanelOpen(isOpen: Boolean) {
        if (isFilterPanelOpen.value == isOpen) {
            return
        }

        isFilterPanelOpen.value = isOpen

        if (!isOpen) {
            resetTagSelection()
        }

        if (query.value.isBlank()) {
            viewModelScope.launch {
                refreshHomeFeed()
            }
        }
    }

    private suspend fun rerunSearch() {
        val currentQuery = query.value.trim()
        if (currentQuery.isBlank()) {
            return
        }

        isSearchLoading.value = true
        searchError.value = null

        runCatching {
            repository.search(currentQuery, limit = 50u).map { it.toUiNote() }
        }.fold(
            onSuccess = {
                searchResults.value = it
                isSearchLoading.value = false
            },
            onFailure = { throwable ->
                searchResults.value = emptyList()
                isSearchLoading.value = false
                searchError.value = throwable.message ?: "Search failed"
            },
        )
    }

    private fun currentHomeFilter(): NoteFeedFilter {
        val currentTags = availableTags.value
        val effectiveUnselectedTags = unselectedTags.value.intersect(currentTags.toSet())
        return HomeFilterState(
            availableTags = currentTags,
            unselectedTags = effectiveUnselectedTags,
            isUntaggedUnselected = isUntaggedUnselected.value,
            isFilterPanelOpen = isFilterPanelOpen.value,
        ).toFeedFilter()
    }

    private fun shouldUseFilteredPortal(filter: NoteFeedFilter): Boolean =
        filter.tagFilterEnabled

    private fun ensureFilteredPortal(filter: NoteFeedFilter): CursorPortal<NoteRecord> {
        if (filteredPortal == null || filteredPortalKey != filter) {
            filteredPortalKey = filter
            filteredPortal = repository.openFilteredPortal(filter, limit = pageLimit)
            filteredPortalState.value = filteredPortal!!.state.value
        }

        return filteredPortal!!
    }

    private fun invalidateHomePortals() {
        recentPortal.invalidate()
        recentSessionsPortal.invalidate()
        filteredPortal?.invalidate()
    }

    private suspend fun refreshAvailableTags() {
        runCatching {
            repository.getAllTags()
        }.onSuccess { tags ->
            availableTags.value = tags
            unselectedTags.value = unselectedTags.value.intersect(tags.toSet())
            feedError.value = null
        }.onFailure { throwable ->
            feedError.value = throwable.message ?: "Unable to load tags"
        }
    }

    private suspend fun refreshHomeFeed() {
        val filter = currentHomeFilter()

        if (isFilterPanelOpen.value) {
            if (shouldUseFilteredPortal(filter)) {
                val portal = ensureFilteredPortal(filter)
                portal.refresh()
                filteredPortalState.value = portal.state.value
            } else {
                recentPortal.refresh()
            }
        } else {
            recentSessionsPortal.refresh()
        }
    }

    private suspend fun loadMoreHomeFeed() {
        val filter = currentHomeFilter()

        if (isFilterPanelOpen.value) {
            if (shouldUseFilteredPortal(filter)) {
                val portal = ensureFilteredPortal(filter)
                if (portal.state.value.hasMore) {
                    portal.loadNext()
                    filteredPortalState.value = portal.state.value
                }
            } else if (recentPortal.state.value.hasMore) {
                recentPortal.loadNext()
            }
        } else if (recentSessionsPortal.state.value.hasMore) {
            recentSessionsPortal.loadNext()
        }
    }

    private fun triggerFilterRefresh() {
        if (query.value.isBlank()) {
            viewModelScope.launch {
                refreshHomeFeed()
            }
        }
    }

    private fun resetTagSelection() {
        unselectedTags.value = emptySet()
        isUntaggedUnselected.value = false
        filteredPortal = null
        filteredPortalKey = null
        filteredPortalState.value = PortalState()
    }
}