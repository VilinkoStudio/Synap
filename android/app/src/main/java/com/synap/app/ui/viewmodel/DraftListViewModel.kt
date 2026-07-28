package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.model.NoteDraftRecord
import com.synap.app.data.repository.SynapRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class DraftListUiState(
    val drafts: List<NoteDraftRecord> = emptyList(),
    val isLoading: Boolean = true,
    val errorMessage: String? = null,
)

@HiltViewModel
class DraftListViewModel @Inject constructor(
    private val repository: SynapRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(DraftListUiState())
    val uiState: StateFlow<DraftListUiState> = _uiState.asStateFlow()

    init { refresh() }

    fun refresh() {
        viewModelScope.launch {
            runCatching { repository.listDrafts().filter(NoteDraftRecord::persisted) }
                .fold(
                    onSuccess = { drafts -> _uiState.value = DraftListUiState(drafts, false) },
                    onFailure = { error ->
                        _uiState.value = DraftListUiState(
                            isLoading = false,
                            errorMessage = error.message ?: "Failed to load drafts",
                        )
                    },
                )
        }
    }

    fun delete(draftId: String) {
        viewModelScope.launch {
            runCatching { repository.discardDraft(draftId) }
            refresh()
        }
    }

    fun clear() {
        viewModelScope.launch {
            val drafts = _uiState.value.drafts
            drafts.forEach { runCatching { repository.discardDraft(it.id) } }
            refresh()
        }
    }
}
