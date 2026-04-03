package com.synap.app.ui.viewmodel

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.repository.SynapRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

sealed interface EditorMode {
    data object Create : EditorMode
    data class Reply(val parentId: String, val parentSummary: String?) : EditorMode
    data class Edit(val noteId: String) : EditorMode
}

sealed interface EditorEvent {
    data class Saved(val noteId: String, val mode: EditorMode) : EditorEvent
}

data class EditorUiState(
    val mode: EditorMode = EditorMode.Create,
    val content: String = "",
    val tags: List<String> = emptyList(),
    val isLoading: Boolean = false,
    val isSaving: Boolean = false,
    val errorMessage: String? = null,
)

@HiltViewModel
class EditorViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    private val repository: SynapRepository,
) : ViewModel() {
    private val mode: EditorMode = when {
        !savedStateHandle.get<String>("editNoteId").isNullOrBlank() -> {
            EditorMode.Edit(checkNotNull(savedStateHandle["editNoteId"]))
        }
        !savedStateHandle.get<String>("parentId").isNullOrBlank() -> {
            EditorMode.Reply(
                parentId = checkNotNull(savedStateHandle["parentId"]),
                parentSummary = savedStateHandle["parentSummary"],
            )
        }
        else -> EditorMode.Create
    }

    private val _uiState = MutableStateFlow(EditorUiState(mode = mode, isLoading = mode is EditorMode.Edit))
    val uiState: StateFlow<EditorUiState> = _uiState.asStateFlow()
    private val _events = MutableSharedFlow<EditorEvent>()
    val events = _events.asSharedFlow()

    init {
        if (mode is EditorMode.Edit) {
            viewModelScope.launch {
                runCatching {
                    repository.getNote(mode.noteId)
                }.fold(
                    onSuccess = { note ->
                        _uiState.value = _uiState.value.copy(
                            content = note.content,
                            tags = note.tags,
                            isLoading = false,
                            errorMessage = null,
                        )
                    },
                    onFailure = { throwable ->
                        _uiState.value = _uiState.value.copy(
                            isLoading = false,
                            errorMessage = throwable.message ?: "Failed to load note",
                        )
                    },
                )
            }
        }
    }

    fun updateContent(value: String) {
        _uiState.update { it.copy(content = value, errorMessage = null) }
    }

    fun addTag(value: String) {
        val normalized = value.trim()
        if (normalized.isEmpty()) {
            return
        }
        _uiState.update { state ->
            state.copy(tags = (state.tags + normalized).distinct(), errorMessage = null)
        }
    }

    fun updateTag(index: Int, value: String) {
        _uiState.update { state ->
            if (index !in state.tags.indices) {
                state
            } else {
                state.copy(
                    tags = state.tags.toMutableList().apply {
                        this[index] = value.trim()
                    }.filter { it.isNotEmpty() }.distinct(),
                    errorMessage = null,
                )
            }
        }
    }

    fun removeTag(index: Int) {
        _uiState.update { state ->
            if (index !in state.tags.indices) {
                state
            } else {
                state.copy(
                    tags = state.tags.toMutableList().apply { removeAt(index) },
                )
            }
        }
    }

    fun save() {
        val content = uiState.value.content.trim()
        if (content.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "正文不能为空") }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isSaving = true, errorMessage = null) }

            runCatching {
                when (val currentMode = uiState.value.mode) {
                    EditorMode.Create -> repository.createNote(content, uiState.value.tags)
                    is EditorMode.Reply -> repository.replyToNote(currentMode.parentId, content, uiState.value.tags)
                    is EditorMode.Edit -> repository.editNote(currentMode.noteId, content, uiState.value.tags)
                }
            }.fold(
                onSuccess = { note ->
                    _uiState.update { it.copy(isSaving = false) }
                    _events.emit(EditorEvent.Saved(note.id, uiState.value.mode))
                },
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isSaving = false,
                            errorMessage = throwable.message ?: "Failed to save note",
                        )
                    }
                },
            )
        }
    }
}
