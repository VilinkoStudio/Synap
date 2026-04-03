package com.synap.app.ui.viewmodel

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.repository.SynapMutation
import com.synap.app.data.repository.SynapRepository
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.toUiNote
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

sealed interface DetailEvent {
    data object NavigateBackAfterDelete : DetailEvent
}

data class DetailUiState(
    val note: Note? = null,
    val origins: List<Note> = emptyList(),
    val previousVersions: List<Note> = emptyList(),
    val nextVersions: List<Note> = emptyList(),
    val replies: List<Note> = emptyList(),
    val isLoading: Boolean = true,
    val repliesLoading: Boolean = false,
    val repliesHasMore: Boolean = false,
    val errorMessage: String? = null,
)

private data class DetailSnapshot(
    val note: Note? = null,
    val origins: List<Note> = emptyList(),
    val previousVersions: List<Note> = emptyList(),
    val nextVersions: List<Note> = emptyList(),
    val isLoading: Boolean = true,
    val errorMessage: String? = null,
)

@HiltViewModel
class DetailViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    private val repository: SynapRepository,
) : ViewModel() {
    private val noteId: String = checkNotNull(savedStateHandle["noteId"])
    private val repliesPortal = repository.openRepliesPortal(parentId = noteId, limit = 20u)
    private val snapshot = MutableStateFlow(DetailSnapshot())
    private val _events = MutableSharedFlow<DetailEvent>()
    val events = _events.asSharedFlow()

    val uiState: StateFlow<DetailUiState> = combine(
        snapshot,
        repliesPortal.state,
    ) { currentSnapshot, replies ->
        DetailUiState(
            note = currentSnapshot.note,
            origins = currentSnapshot.origins,
            previousVersions = currentSnapshot.previousVersions,
            nextVersions = currentSnapshot.nextVersions,
            replies = replies.items.map { item ->
                item.toUiNote(parentSummary = currentSnapshot.note?.content)
            },
            isLoading = currentSnapshot.isLoading,
            repliesLoading = replies.isLoading,
            repliesHasMore = replies.hasMore,
            errorMessage = currentSnapshot.errorMessage ?: replies.error?.message,
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = DetailUiState(),
    )

    init {
        refreshAll()

        viewModelScope.launch {
            repository.mutations.collect { mutation ->
                when (mutation) {
                    is SynapMutation.Replied -> if (mutation.parentId == noteId) {
                        repliesPortal.refresh()
                    }
                    is SynapMutation.Edited -> if (mutation.oldId == noteId || mutation.newId == noteId) {
                        refreshAll()
                    }
                    is SynapMutation.Deleted -> if (mutation.targetId == noteId) {
                        _events.emit(DetailEvent.NavigateBackAfterDelete)
                    }
                    is SynapMutation.Restored -> if (mutation.targetId == noteId) {
                        refreshAll()
                    }
                    is SynapMutation.Created -> Unit
                }
            }
        }
    }

    fun refreshAll() {
        viewModelScope.launch {
            loadSnapshot()
            repliesPortal.refresh()
        }
    }

    fun loadMoreReplies() {
        viewModelScope.launch {
            repliesPortal.loadNext()
        }
    }

    fun deleteCurrentNote() {
        viewModelScope.launch {
            runCatching {
                repository.deleteNote(noteId)
            }.onFailure { throwable ->
                snapshot.value = snapshot.value.copy(
                    errorMessage = throwable.message ?: "Failed to delete note",
                )
            }
        }
    }

    private suspend fun loadSnapshot() {
        snapshot.value = snapshot.value.copy(isLoading = true, errorMessage = null)

        runCatching {
            coroutineScope {
                val noteDeferred = async { repository.getNote(noteId).toUiNote() }
                val originsDeferred = async { repository.getOrigins(noteId).map { it.toUiNote() } }
                val previousDeferred = async { repository.getPreviousVersions(noteId).map { it.toUiNote() } }
                val nextDeferred = async { repository.getNextVersions(noteId).map { it.toUiNote() } }

                DetailSnapshot(
                    note = noteDeferred.await(),
                    origins = originsDeferred.await(),
                    previousVersions = previousDeferred.await(),
                    nextVersions = nextDeferred.await(),
                    isLoading = false,
                    errorMessage = null,
                )
            }
        }.fold(
            onSuccess = { snapshot.value = it },
            onFailure = { throwable ->
                snapshot.value = DetailSnapshot(
                    isLoading = false,
                    errorMessage = throwable.message ?: "Failed to load note",
                )
            },
        )
    }
}
