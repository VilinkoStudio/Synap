package com.synap.app.ui.viewmodel

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.NoteFeedStatus
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.TimelineDensityPointRecord
import com.synap.app.data.portal.CursorPortal
import com.synap.app.data.portal.PortalState
import com.synap.app.data.repository.SynapRepository
import com.synap.app.ui.model.HomeDisplayPrefs
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.SearchResultNote
import com.synap.app.ui.model.toUiNote
import com.synap.app.ui.model.toUiSearchResultNote
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import java.time.LocalDate
import java.time.ZoneId

data class HomeUiState(
    val query: String = "",
    val notes: List<Note> = emptyList(),
    val searchResults: List<SearchResultNote> = emptyList(),
    val isLoading: Boolean = true,
    val hasMore: Boolean = false,
    val isSearchMode: Boolean = false,
    val showTagBar: Boolean = HomeDisplayPrefs.DEFAULT_SHOW_TAG_BAR,
    val showTimeGroups: Boolean = true,
    val showTimelineJumpTool: Boolean = HomeDisplayPrefs.DEFAULT_SHOW_TIMELINE_JUMP_TOOL,
    val availableTags: List<String> = emptyList(),
    val unselectedTags: Set<String> = emptySet(),
    val isUntaggedUnselected: Boolean = false,
    val errorMessage: String? = null,
    val timelineBrowser: TimelineBrowserUiState = TimelineBrowserUiState(),
)

data class TimelineBrowserUiState(
    val selectedDate: LocalDate = LocalDate.now(),
    val densityPoints: List<TimelineDensityPoint> = emptyList(),
    val isDensityLoading: Boolean = false,
    val densityErrorMessage: String? = null,
)

data class TimelineDensityPoint(
    val startedAt: Long,
    val endedAt: Long,
    val noteCount: Int,
)

private data class TimelineAnchor(
    val date: LocalDate,
)

private data class HomeQueryState(
    val query: String,
    val searchResults: List<SearchResultNote>,
    val isSearchLoading: Boolean,
    val searchError: String?,
)

private data class HomeFilterState(
    val availableTags: List<String>,
    val unselectedTags: Set<String>,
    val isUntaggedUnselected: Boolean,
    val showTagBar: Boolean,
) {
    fun toFeedFilter(): NoteFeedFilter {
        val selectedTags = availableTags.filterNot { it in unselectedTags }
        return NoteFeedFilter(
            selectedTags = selectedTags,
            includeUntagged = !isUntaggedUnselected,
            tagFilterEnabled = showTagBar && (unselectedTags.isNotEmpty() || isUntaggedUnselected),
            status = NoteFeedStatus.Normal,
            groupSessions = true,
        )
    }
}

private data class HomeFeedState(
    val notes: List<Note>,
    val isLoading: Boolean,
    val hasMore: Boolean,
    val showTagBar: Boolean,
    val availableTags: List<String>,
    val unselectedTags: Set<String>,
    val isUntaggedUnselected: Boolean,
    val errorMessage: String?,
)

private data class HomeDisplayState(
    val showTimeGroups: Boolean,
    val showTimelineJumpTool: Boolean,
)

@HiltViewModel
class HomeViewModel @Inject constructor(
    private val repository: SynapRepository,
    @ApplicationContext context: Context,
) : ViewModel() {
    private val prefs = context.getSharedPreferences("synap_prefs", Context.MODE_PRIVATE)
    private val pageLimit = 20u
    private val query = MutableStateFlow("")
    private val searchResults = MutableStateFlow<List<SearchResultNote>>(emptyList())
    private val isSearchLoading = MutableStateFlow(false)
    private val searchError = MutableStateFlow<String?>(null)
    private val feedError = MutableStateFlow<String?>(null)
    private val availableTags = MutableStateFlow<List<String>>(emptyList())
    private val unselectedTags = MutableStateFlow<Set<String>>(emptySet())
    private val isUntaggedUnselected = MutableStateFlow(false)
    private val showTagBar = MutableStateFlow(
        prefs.getBoolean(
            HomeDisplayPrefs.SHOW_TAG_BAR,
            HomeDisplayPrefs.DEFAULT_SHOW_TAG_BAR,
        )
    )
    private val showTimeGroups = MutableStateFlow(
        prefs.getBoolean(HomeDisplayPrefs.SHOW_TIME_GROUPS, HomeDisplayPrefs.DEFAULT_SHOW_TIME_GROUPS)
    )
    private val showTimelineJumpTool = MutableStateFlow(
        prefs.getBoolean(
            HomeDisplayPrefs.SHOW_TIMELINE_JUMP_TOOL,
            HomeDisplayPrefs.DEFAULT_SHOW_TIMELINE_JUMP_TOOL,
        )
    )
    private val timelinePortalState = MutableStateFlow(PortalState<NoteRecord>())
    private var timelinePortal: CursorPortal<NoteRecord>? = null
    private var timelinePortalKey: Pair<NoteFeedFilter, TimelineAnchor?>? = null
    private val browserSelectedDate = MutableStateFlow(LocalDate.now())
    private val densityPoints = MutableStateFlow<List<TimelineDensityPointRecord>>(emptyList())
    private val isDensityLoading = MutableStateFlow(false)
    private val densityError = MutableStateFlow<String?>(null)
    private val timelineAnchor = MutableStateFlow<TimelineAnchor?>(null)

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
        showTagBar,
    ) { currentTags, currentUnselectedTags, currentIsUntaggedUnselected, currentShowTagBar ->
        HomeFilterState(
            availableTags = currentTags,
            unselectedTags = currentUnselectedTags.intersect(currentTags.toSet()),
            isUntaggedUnselected = currentIsUntaggedUnselected,
            showTagBar = currentShowTagBar,
        )
    }

    private val homeFeedState = combine(
        timelinePortalState,
        filterState,
    ) { timeline, currentFilterState ->
        val homeNotes = timeline.items.map { record -> record.toUiNote() }

        HomeFeedState(
            notes = homeNotes,
            isLoading = timeline.isLoading,
            hasMore = timeline.hasMore,
            showTagBar = currentFilterState.showTagBar,
            availableTags = currentFilterState.availableTags,
            unselectedTags = currentFilterState.unselectedTags,
            isUntaggedUnselected = currentFilterState.isUntaggedUnselected,
            errorMessage = timeline.error?.message,
        )
    }

    private val timelineBrowserState = combine(
        browserSelectedDate,
        densityPoints,
        isDensityLoading,
        densityError,
    ) { currentBrowserDate, currentDensityPoints, currentDensityLoading, currentDensityError ->
        TimelineBrowserUiState(
            selectedDate = currentBrowserDate,
            densityPoints = currentDensityPoints.map { point ->
                TimelineDensityPoint(
                    startedAt = point.startedAt,
                    endedAt = point.endedAt,
                    noteCount = point.noteCount,
                )
            },
            isDensityLoading = currentDensityLoading,
            densityErrorMessage = currentDensityError,
        )
    }

    private val homeDisplayState = combine(
        showTimeGroups,
        showTimelineJumpTool,
    ) { currentShowTimeGroups, currentShowTimelineJumpTool ->
        HomeDisplayState(
            showTimeGroups = currentShowTimeGroups,
            showTimelineJumpTool = currentShowTimelineJumpTool,
        )
    }

    val uiState: StateFlow<HomeUiState> = combine(
        homeFeedState,
        queryState,
        feedError,
        timelineBrowserState,
        homeDisplayState,
    ) { currentHomeFeed, currentState, currentFeedError, currentTimelineBrowser, currentDisplayState ->
        val searchMode = currentState.query.isNotBlank()

        HomeUiState(
            query = currentState.query,
            notes = if (searchMode) currentState.searchResults.map { it.note } else currentHomeFeed.notes,
            searchResults = currentState.searchResults,
            isLoading = if (searchMode) currentState.isSearchLoading else currentHomeFeed.isLoading,
            hasMore = if (searchMode) false else currentHomeFeed.hasMore,
            isSearchMode = searchMode,
            showTagBar = currentHomeFeed.showTagBar,
            showTimeGroups = currentDisplayState.showTimeGroups && !searchMode,
            showTimelineJumpTool = currentDisplayState.showTimelineJumpTool && !searchMode,
            availableTags = currentHomeFeed.availableTags,
            unselectedTags = currentHomeFeed.unselectedTags,
            isUntaggedUnselected = currentHomeFeed.isUntaggedUnselected,
            errorMessage = if (searchMode) {
                currentState.searchError
            } else {
                currentHomeFeed.errorMessage ?: currentFeedError
            },
            timelineBrowser = currentTimelineBrowser,
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

    fun openTimelineBrowser() {
        viewModelScope.launch {
            refreshTimelineDensity()
        }
    }

    fun selectTimelineBrowserDate(date: LocalDate) {
        browserSelectedDate.value = date
        timelineAnchor.value = TimelineAnchor(date)
        timelinePortal = null
        timelinePortalKey = null
        timelinePortalState.value = PortalState()
        viewModelScope.launch {
            refreshHomeFeed()
            refreshTimelineDensity()
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

    fun setHomeDisplayOptions(
        tagBarEnabled: Boolean,
        timeGroupsEnabled: Boolean,
        timelineJumpToolEnabled: Boolean,
    ) {
        if (
            showTagBar.value == tagBarEnabled &&
            showTimeGroups.value == timeGroupsEnabled &&
            showTimelineJumpTool.value == timelineJumpToolEnabled
        ) {
            return
        }

        showTagBar.value = tagBarEnabled
        showTimeGroups.value = timeGroupsEnabled
        showTimelineJumpTool.value = timelineJumpToolEnabled
        prefs.edit()
            .putBoolean(HomeDisplayPrefs.SHOW_TAG_BAR, tagBarEnabled)
            .putBoolean(HomeDisplayPrefs.SHOW_TIME_GROUPS, timeGroupsEnabled)
            .putBoolean(HomeDisplayPrefs.SHOW_TIMELINE_JUMP_TOOL, timelineJumpToolEnabled)
            .apply()

        if (query.value.isBlank()) {
            viewModelScope.launch {
                refreshHomeFeed()
                if (timelineJumpToolEnabled) {
                    refreshTimelineDensity()
                }
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
            repository.searchFusion(
                currentQuery,
                limit = 50u,
                fuzzyLimit = null,
                semanticLimit = 10u,
            ).map { it.toUiSearchResultNote() }
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
            showTagBar = showTagBar.value,
        ).toFeedFilter()
    }

    private fun ensureTimelinePortal(filter: NoteFeedFilter): CursorPortal<NoteRecord> {
        val anchor = timelineAnchor.value
        val key = filter to anchor
        if (timelinePortal == null || timelinePortalKey != key) {
            timelinePortalKey = key
            timelinePortal = if (anchor == null) {
                repository.openTimelinePortal(filter, limit = pageLimit)
            } else {
                repository.openTimelineAroundPortal(
                    filter = filter,
                    timestampMs = dateEndTimestampMs(anchor.date),
                    limit = pageLimit,
                )
            }
            timelinePortalState.value = timelinePortal!!.state.value
        }

        return timelinePortal!!
    }

    private fun invalidateHomePortals() {
        timelinePortal?.invalidate()
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
        val portal = ensureTimelinePortal(filter)
        portal.refresh()
        timelinePortalState.value = portal.state.value
    }

    private suspend fun refreshTimelineDensity() {
        val selectedDate = browserSelectedDate.value
        val startDate = selectedDate.minusDays(6)
        val endDate = selectedDate.plusDays(8)
        val zone = ZoneId.systemDefault()
        val startMs = startDate.atStartOfDay(zone).toInstant().toEpochMilli().toULong()
        val endMs = endDate.atStartOfDay(zone).toInstant().toEpochMilli().toULong()
        val dayMs = 24UL * 60UL * 60UL * 1000UL

        isDensityLoading.value = true
        densityError.value = null
        runCatching {
            repository.getTimelineDensity(
                filter = currentHomeFilter().copy(groupSessions = false),
                startMs = startMs,
                endMs = endMs,
                bucketMs = dayMs,
            )
        }.fold(
            onSuccess = { points ->
                densityPoints.value = points
                isDensityLoading.value = false
            },
            onFailure = { throwable ->
                densityPoints.value = emptyList()
                isDensityLoading.value = false
                densityError.value = throwable.message ?: "Unable to load timeline density"
            },
        )
    }

    private suspend fun loadMoreHomeFeed() {
        val filter = currentHomeFilter()
        val portal = ensureTimelinePortal(filter)
        if (portal.state.value.hasMore) {
            portal.loadNext()
            timelinePortalState.value = portal.state.value
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
        timelinePortal = null
        timelinePortalKey = null
        timelinePortalState.value = PortalState()
    }

    private fun dateEndTimestampMs(date: LocalDate): ULong {
        val zone = ZoneId.systemDefault()
        val millis = date
            .plusDays(1)
            .atStartOfDay(zone)
            .toInstant()
            .toEpochMilli()
            .saturatingMinus(1L)
        return millis.toULong()
    }
}

private fun Long.saturatingMinus(value: Long): Long =
    if (this < Long.MIN_VALUE + value) Long.MIN_VALUE else this - value
