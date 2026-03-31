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
    val showDeleted: Boolean = false, // 保留字段以防报错，但不再用于切换逻辑
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

    // 移除了 showDeleted 的合并逻辑
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

        // --- 核心修改：同时获取正常笔记和已删除笔记，并合并到一个列表中 ---
        val recentNotes = recent.items.map { it.toUiNote(isDeleted = false) }
        val deletedNotes = deleted.items.map { it.toUiNote(isDeleted = true) }
        val combinedNotes = recentNotes + deletedNotes

        HomeUiState(
            query = currentState.query,
            notes = if (searchMode) currentState.searchResults else combinedNotes,
            // 只要其中一个还在加载，就显示 Loading
            isLoading = if (searchMode) {
                currentState.isSearchLoading
            } else {
                recent.isLoading || deleted.isLoading
            },
            // 只要其中一个还有更多数据，就允许继续下滑加载
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

    // --- 修改：触底时同时尝试加载两种笔记的下一页 ---
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