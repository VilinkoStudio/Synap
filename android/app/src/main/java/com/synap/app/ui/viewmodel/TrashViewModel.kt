package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.repository.SynapMutation
import com.synap.app.data.repository.SynapRepository
import com.synap.app.ui.model.Note
import com.synap.app.ui.model.toUiNote
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class TrashUiState(
    val notes: List<Note> = emptyList(),
    val isLoading: Boolean = true,
    val hasMore: Boolean = false,
    val errorMessage: String? = null,
)

@HiltViewModel
class TrashViewModel @Inject constructor(
    private val repository: SynapRepository,
) : ViewModel() {
    private val deletedPortal = repository.openDeletedPortal(limit = 20u)
    private val feedError = MutableStateFlow<String?>(null)

    val uiState: StateFlow<TrashUiState> = combine(
        deletedPortal.state,
        feedError,
    ) { deleted, currentFeedError ->
        TrashUiState(
            notes = deleted.items.map { it.toUiNote() },
            isLoading = deleted.isLoading,
            hasMore = deleted.hasMore,
            errorMessage = deleted.error?.message ?: currentFeedError,
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = TrashUiState(),
    )

    init {
        refresh()

        viewModelScope.launch {
            repository.mutations.collect { mutation ->
                when (mutation) {
                    is SynapMutation.Deleted, is SynapMutation.Restored, is SynapMutation.Imported -> {
                        deletedPortal.invalidate()
                        refresh()
                    }
                    is SynapMutation.Created,
                    is SynapMutation.Replied,
                    is SynapMutation.Edited -> Unit
                }
            }
        }
    }

    fun refresh() {
        viewModelScope.launch {
            deletedPortal.refresh()
        }
    }

    fun loadMore() {
        viewModelScope.launch {
            if (deletedPortal.state.value.hasMore) {
                deletedPortal.loadNext()
            }
        }
    }

    fun restoreNote(note: Note) {
        if (!note.isDeleted) {
            return
        }

        viewModelScope.launch {
            runCatching {
                repository.restoreNote(note.id)
                feedError.value = null
            }.onFailure { throwable ->
                feedError.value = throwable.message ?: "Unable to restore note"
            }
        }
    }
}
