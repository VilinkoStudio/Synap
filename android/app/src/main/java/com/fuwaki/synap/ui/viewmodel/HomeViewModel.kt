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
    val showDeleted: Boolean,
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
    private val showDeleted = MutableStateFlow(false)
    private val searchResults = MutableStateFlow<List<Note>>(emptyList())
    private val isSearchLoading = MutableStateFlow(false)
    private val searchError = MutableStateFlow<String?>(null)
    private val queryState = combine(
        query,
        showDeleted,
        searchResults,
        isSearchLoading,
        searchError,
    ) { currentQuery, currentShowDeleted, currentSearchResults, searching, currentSearchError ->
        HomeQueryState(
            query = currentQuery,
            showDeleted = currentShowDeleted,
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
        val portalState = if (currentState.showDeleted) deleted else recent
        val portalNotes = if (currentState.showDeleted) {
            deleted.items.map { it.toUiNote(isDeleted = true) }
        } else {
            recent.items.map { it.toUiNote() }
        }

        HomeUiState(
            query = currentState.query,
            notes = if (searchMode) currentState.searchResults else portalNotes,
            isLoading = if (searchMode) {
                currentState.isSearchLoading
            } else {
                portalState.isLoading
            },
            hasMore = if (searchMode) {
                false
            } else {
                portalState.hasMore
            },
            isSearchMode = searchMode,
            showDeleted = currentState.showDeleted,
            errorMessage = currentState.searchError ?: portalState.error?.message,
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

    fun toggleDeletedFeed() {
        showDeleted.value = !showDeleted.value
        if (query.value.isBlank()) {
            refresh()
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
            if (showDeleted.value) {
                deletedPortal.loadNext()
            } else {
                recentPortal.loadNext()
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
