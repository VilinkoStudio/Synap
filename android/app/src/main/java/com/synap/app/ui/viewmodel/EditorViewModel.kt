package com.synap.app.ui.viewmodel

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.repository.SynapRepository
import com.synap.app.data.service.DraftRecord
import com.synap.app.data.service.DraftStore
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import com.synap.app.ui.util.NoteColorUtil
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
    val noteColorHue: Float? = null,
    val isLoading: Boolean = false,
    val isSaving: Boolean = false,
    val isRecommendingTags: Boolean = false,
    val recommendedTags: List<String> = emptyList(),
    val errorMessage: String? = null,
)

@HiltViewModel
class EditorViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    private val repository: SynapRepository,
    private val draftStore: DraftStore,
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
    private var recommendTagsJob: Job? = null
    private var recommendedTagCandidates: List<String> = emptyList()
    private var lastRecommendedContent: String? = null
    private var autoSaveJob: Job? = null
    private var currentDraftId: String? = null
    private var initialContent: String = ""
    private var hasBeenModified = false

    init {
        if (mode is EditorMode.Edit) {
            viewModelScope.launch {
                runCatching {
                    repository.getNote(mode.noteId)
                }.fold(
                    onSuccess = { note ->
                        val colorHue = NoteColorUtil.extractColorHue(note.tags)
                        val displayTags = NoteColorUtil.filterDisplayTags(note.tags)
                        _uiState.value = _uiState.value.copy(
                            content = note.content,
                            tags = displayTags,
                            noteColorHue = colorHue,
                            isLoading = false,
                            errorMessage = null,
                        )
                        scheduleTagRecommendations(note.content)
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
        if (value != initialContent) {
            hasBeenModified = true
        }
        scheduleTagRecommendations(value)
        scheduleAutoSave()
    }

    fun addTag(value: String) {
        val normalized = value.trim()
        if (normalized.isEmpty()) {
            return
        }
        _uiState.update { state ->
            val updatedTags = (state.tags + normalized).distinct()
            state.copy(
                tags = updatedTags,
                recommendedTags = filterRecommendedTags(updatedTags),
                errorMessage = null,
            )
        }
        hasBeenModified = true
        scheduleAutoSave()
    }

    fun updateTag(index: Int, value: String) {
        _uiState.update { state ->
            if (index !in state.tags.indices) {
                state
            } else {
                val updatedTags = state.tags.toMutableList().apply {
                    this[index] = value.trim()
                }.filter { it.isNotEmpty() }.distinct()
                state.copy(
                    tags = updatedTags,
                    recommendedTags = filterRecommendedTags(updatedTags),
                    errorMessage = null,
                )
            }
        }
        hasBeenModified = true
        scheduleAutoSave()
    }

    fun removeTag(index: Int) {
        _uiState.update { state ->
            if (index !in state.tags.indices) {
                state
            } else {
                val updatedTags = state.tags.toMutableList().apply { removeAt(index) }
                state.copy(
                    tags = updatedTags,
                    recommendedTags = filterRecommendedTags(updatedTags),
                )
            }
        }
        hasBeenModified = true
        scheduleAutoSave()
    }

    fun setNoteColorHue(hue: Float?) {
        _uiState.update { it.copy(noteColorHue = hue) }
        hasBeenModified = true
        scheduleAutoSave()
    }

    fun save() {
        val content = uiState.value.content.trim()
        if (content.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "正文不能为空") }
            return
        }

        val state = uiState.value
        val storageTags = NoteColorUtil.prepareStorageTags(state.tags, state.noteColorHue)

        viewModelScope.launch {
            _uiState.update { it.copy(isSaving = true, errorMessage = null) }

            runCatching {
                when (val currentMode = state.mode) {
                    EditorMode.Create -> repository.createNote(content, storageTags)
                    is EditorMode.Reply -> repository.replyToNote(currentMode.parentId, content, storageTags)
                    is EditorMode.Edit -> repository.editNote(currentMode.noteId, content, storageTags)
                }
            }.fold(
                onSuccess = { note ->
                    _uiState.update { it.copy(isSaving = false) }
                    clearCurrentDraft()
                    _events.emit(EditorEvent.Saved(note.id, state.mode))
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

    fun hasUnsavedChanges(): Boolean {
        if (!hasBeenModified) return false
        val state = uiState.value
        return state.content.trim().isNotEmpty()
    }

    fun saveDraftManually() {
        saveDraft("manual")
    }

    private fun scheduleAutoSave() {
        autoSaveJob?.cancel()
        autoSaveJob = viewModelScope.launch {
            delay(AUTO_SAVE_DEBOUNCE_MS)
            saveDraft("auto")
        }
    }

    private fun saveDraft(reason: String) {
        if (!draftStore.isEnabled()) return
        val state = uiState.value
        val content = state.content.trim()
        if (content.isEmpty()) return

        val draft = DraftRecord(
            id = currentDraftId ?: java.util.UUID.randomUUID().toString(),
            content = content,
            tags = state.tags,
            noteColorHue = state.noteColorHue,
            mode = when (state.mode) {
                EditorMode.Create -> "create"
                is EditorMode.Reply -> "reply"
                is EditorMode.Edit -> "edit"
            },
            parentId = (state.mode as? EditorMode.Reply)?.parentId,
            parentSummary = (state.mode as? EditorMode.Reply)?.parentSummary,
            editNoteId = (state.mode as? EditorMode.Edit)?.noteId,
            savedAt = System.currentTimeMillis(),
            reason = reason,
            status = "editing", // 自动保存的草稿状态为编辑中
        )

        if (currentDraftId == null) {
            currentDraftId = draft.id
        }

        draftStore.save(draft)
    }

    private fun clearCurrentDraft() {
        currentDraftId?.let { draftStore.delete(it) }
        currentDraftId = null
    }

    fun getCurrentDraftId(): String? {
        return currentDraftId
    }

    fun isContentMatchingLatestDraft(): Boolean {
        val latestDraft = draftStore.getLatestDraft() ?: return false
        val currentContent = uiState.value.content.trim()
        return latestDraft.content.trim() == currentContent && currentContent.isNotEmpty()
    }

    fun markDraftAsRead(draftId: String) {
        draftStore.updateStatus(draftId, "read")
    }

    private fun scheduleTagRecommendations(content: String) {
        recommendTagsJob?.cancel()

        val normalized = content.trim()
        if (normalized.isEmpty()) {
            clearTagRecommendations(resetCache = true)
            return
        }

        recommendTagsJob = viewModelScope.launch {
            delay(TAG_RECOMMENDATION_DEBOUNCE_MS)

            if (normalized != _uiState.value.content.trim()) {
                return@launch
            }

            if (normalized == lastRecommendedContent) {
                _uiState.update { state ->
                    state.copy(recommendedTags = filterRecommendedTags(state.tags))
                }
                return@launch
            }

            _uiState.update { it.copy(isRecommendingTags = true) }

            try {
                recommendedTagCandidates = repository
                    .recommendTag(normalized, TAG_RECOMMENDATION_LIMIT)
                    .map(String::trim)
                    .filter(String::isNotEmpty)
                    .distinct()
                lastRecommendedContent = normalized

                _uiState.update { state ->
                    state.copy(
                        isRecommendingTags = false,
                        recommendedTags = filterRecommendedTags(state.tags),
                    )
                }
            } catch (cancellation: CancellationException) {
                throw cancellation
            } catch (_: Throwable) {
                clearTagRecommendations(resetCache = true)
            }
        }
    }

    private fun clearTagRecommendations(resetCache: Boolean = false) {
        if (resetCache) {
            recommendedTagCandidates = emptyList()
            lastRecommendedContent = null
        }

        _uiState.update {
            it.copy(
                isRecommendingTags = false,
                recommendedTags = emptyList(),
            )
        }
    }

    private fun filterRecommendedTags(selectedTags: List<String>): List<String> {
        if (recommendedTagCandidates.isEmpty()) {
            return emptyList()
        }

        val selected = selectedTags
            .map(String::trim)
            .filter(String::isNotEmpty)
            .toSet()

        return recommendedTagCandidates.filterNot(selected::contains)
    }

    private companion object {
        private const val TAG_RECOMMENDATION_DEBOUNCE_MS = 350L
        private val TAG_RECOMMENDATION_LIMIT = 6u
        private const val AUTO_SAVE_DEBOUNCE_MS = 3000L // 3 seconds
    }
}
