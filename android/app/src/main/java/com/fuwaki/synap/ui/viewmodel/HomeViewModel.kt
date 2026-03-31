package com.fuwaki.synap.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.fuwaki.synap.data.repository.SynapRepository
import com.fuwaki.synap.ui.model.Note
import com.fuwaki.synap.ui.model.toUiNote
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
    val isLoading: Boolean = true,
    val hasMore: Boolean = false,
    val isSearchMode: Boolean = false,
    val showDeleted: Boolean = false,
    val errorMessage: String? = null,
)

private data class HomeQueryState(
    val query: String,
    val searchResults: List<Note>,
    val isSearchLoading: Boolean,
    val searchError: String?,
)

@HiltViewModel
class HomeViewModel @Inject constructor(
    private val repository: SynapRepository,
) : ViewModel() {
    private val recentPortal = repository.openRecentPortal(limit = 20u)
    private val deletedPortal = repository.openDeletedPortal(limit = 20u)
    private val query = MutableStateFlow("")
    private val searchResults = MutableStateFlow<List<Note>>(emptyList())
    private val isSearchLoading = MutableStateFlow(false)
    private val searchError = MutableStateFlow<String?>(null)

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

    val uiState: StateFlow<HomeUiState> = combine(
        recentPortal.state,
        deletedPortal.state,
        queryState,
    ) { recent, deleted, currentState ->
        val searchMode = currentState.query.isNotBlank()

        val recentNotes = recent.items.map { it.toUiNote(isDeleted = false) }
        val deletedNotes = deleted.items.map { it.toUiNote(isDeleted = true) }

        // --- 核心修复 1：使用 Map 进行硬核去重 ---
        // 因为 recent 和 deleted 是异步查出来的，当笔记状态刚切换时，
        // 极可能在瞬间短暂地同时存在于两边。用 Map 可以完美覆盖旧数据。
        val allNotesMap = mutableMapOf<String, Note>()
        // 正常笔记先进
        recentNotes.forEach { allNotesMap[it.id] = it }
        // 垃圾桶笔记后进。如果存在重叠（说明刚被删除），以后进的 isDeleted=true 为准
        deletedNotes.forEach { allNotesMap[it.id] = it }

        val combinedNotes = allNotesMap.values.toList()

        HomeUiState(
            query = currentState.query,
            notes = if (searchMode) currentState.searchResults else combinedNotes,
            isLoading = if (searchMode) {
                currentState.isSearchLoading
            } else {
                recent.isLoading || deleted.isLoading
            },
            hasMore = if (searchMode) {
                false
            } else {
                recent.hasMore || deleted.hasMore
            },
            isSearchMode = searchMode,
            showDeleted = false,
            errorMessage = currentState.searchError ?: recent.error?.message ?: deleted.error?.message,
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
                if (query.value.isBlank()) {
                    refresh()
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
            recentPortal.refresh()
            deletedPortal.refresh()
        }
    }

    fun loadMore() {
        if (query.value.isNotBlank()) {
            return
        }

        viewModelScope.launch {
            if (recentPortal.state.value.hasMore) {
                recentPortal.loadNext()
            }
            if (deletedPortal.state.value.hasMore) {
                deletedPortal.loadNext()
            }
        }
    }

    fun toggleDeleted(note: Note) {
        viewModelScope.launch {
            runCatching {
                if (note.isDeleted) {
                    repository.restoreNote(note.id)
                } else {
                    repository.deleteNote(note.id)
                }
            }.onFailure { throwable ->
                searchError.value = throwable.message ?: "Unable to update note"
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
}