package com.synap.app.ui.viewmodel

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.repository.SynapRepository
import com.synap.app.data.model.NoteDraftRecord
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
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

sealed interface EditorMode {
    data object Create : EditorMode
    data class Reply(val parentId: String, val parentSummary: String?) : EditorMode
    data class Edit(val noteId: String) : EditorMode
}

sealed interface EditorEvent {
    data class Saved(val noteId: String, val mode: EditorMode) : EditorEvent
    data object Closed : EditorEvent
}

internal fun shouldPersistDraftOnClose(content: String, hasUserChanges: Boolean): Boolean =
    hasUserChanges && content.isNotBlank()

data class EditorUiState(
    val mode: EditorMode = EditorMode.Create,
    val content: String = "",
    val tags: List<String> = emptyList(),
    val noteColorHue: Float? = null,
    val noteColorCss: String? = null,
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
) : ViewModel() {
    private var mode: EditorMode = when {
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
    private val restoredDraftId: String? = savedStateHandle["draftId"]

    private val initialContentArgument: String? = savedStateHandle["initialContent"]
    private val _uiState = MutableStateFlow(EditorUiState(mode = mode, isLoading = true))
    val uiState: StateFlow<EditorUiState> = _uiState.asStateFlow()
    private val _events = MutableSharedFlow<EditorEvent>()
    val events = _events.asSharedFlow()
    private var recommendTagsJob: Job? = null
    private var recommendedTagCandidates: List<String> = emptyList()
    private var lastRecommendedContent: String? = null
    private var autoSaveJob: Job? = null
    private var currentDraft: NoteDraftRecord? = null
    private val draftMutationMutex = Mutex()
    private var initializationJob: Job? = null
    private var hasUserChanges = restoredDraftId != null || !initialContentArgument.isNullOrBlank()
    private var isClosing = false

    init {
        initializationJob = viewModelScope.launch {
            runCatching {
                val draft = restoredDraftId?.let { repository.getDraft(it) } ?: when (val currentMode = mode) {
                    EditorMode.Create -> repository.createDraft()
                    is EditorMode.Reply -> repository.createReplyDraft(currentMode.parentId)
                    is EditorMode.Edit -> repository.createEditDraft(currentMode.noteId)
                }
                val initialContent = initialContentArgument?.takeIf(String::isNotBlank)
                if (initialContent != null && mode !is EditorMode.Edit) {
                    repository.updateDraft(draft.id, content = initialContent)
                } else {
                    draft
                }
            }.fold(
                onSuccess = ::bindDraft,
                onFailure = { throwable ->
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            errorMessage = throwable.message ?: "Failed to create draft",
                        )
                    }
                },
            )
        }
    }

    private fun bindDraft(draft: NoteDraftRecord) {
        mode = when {
            draft.editedFrom != null -> EditorMode.Edit(draft.editedFrom)
            draft.replyTo != null -> EditorMode.Reply(draft.replyTo, null)
            else -> EditorMode.Create
        }
        currentDraft = draft
        _uiState.update {
            it.copy(
                mode = mode,
                content = draft.content,
                tags = draft.tags,
                noteColorHue = NoteColorUtil.extractHueFromCss(draft.color),
                noteColorCss = draft.color,
                isLoading = false,
                errorMessage = null,
            )
        }
        scheduleTagRecommendations(draft.content)
    }

    private fun synchronizeDraft() {
        viewModelScope.launch {
            runCatching {
                draftMutationMutex.withLock {
                    synchronizeCurrentDraft(persist = false)
                }
            }.onFailure { throwable ->
                _uiState.update { it.copy(errorMessage = throwable.message ?: "Failed to update draft") }
            }
        }
    }

    fun updateContent(value: String) {
        _uiState.update { it.copy(content = value, errorMessage = null) }
        hasUserChanges = true
        scheduleTagRecommendations(value)
        synchronizeDraft()
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
        hasUserChanges = true
        synchronizeDraft()
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
        hasUserChanges = true
        synchronizeDraft()
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
        hasUserChanges = true
        synchronizeDraft()
        scheduleAutoSave()
    }

    fun setNoteColorHue(hue: Float?) {
        _uiState.update {
            it.copy(
                noteColorHue = hue,
                noteColorCss = hue?.let(NoteColorUtil::hueToCssHex),
            )
        }
        hasUserChanges = true
        synchronizeDraft()
        scheduleAutoSave()
    }

    fun save() {
        val content = uiState.value.content.trim()
        if (content.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "正文不能为空") }
            return
        }

        val state = uiState.value

        viewModelScope.launch {
            _uiState.update { it.copy(isSaving = true, errorMessage = null) }

            runCatching {
                draftMutationMutex.withLock {
                    val synchronized = synchronizeCurrentDraft(persist = false)
                        ?: error("Draft is not ready")
                    repository.commitDraft(synchronized.id)
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

    private fun scheduleAutoSave() {
        autoSaveJob?.cancel()
        autoSaveJob = viewModelScope.launch {
            delay(AUTO_SAVE_DEBOUNCE_MS)
            persistCurrentDraft()
        }
    }

    private fun persistCurrentDraft() {
        if (_uiState.value.content.trim().isEmpty()) return
        viewModelScope.launch {
            runCatching {
                draftMutationMutex.withLock {
                    synchronizeCurrentDraft(persist = true)
                }
            }.onFailure { throwable ->
                _uiState.update { it.copy(errorMessage = throwable.message ?: "Failed to persist draft") }
            }
        }
    }

    private fun clearCurrentDraft() {
        currentDraft = null
    }

    fun close() {
        if (isClosing || _uiState.value.isSaving) return
        isClosing = true
        autoSaveJob?.cancel()
        viewModelScope.launch {
            initializationJob?.join()
            runCatching {
                draftMutationMutex.withLock {
                    val draft = currentDraft ?: return@withLock
                    val state = _uiState.value
                    if (!shouldPersistDraftOnClose(state.content, hasUserChanges)) {
                        repository.discardDraft(draft.id)
                        currentDraft = null
                    } else {
                        synchronizeCurrentDraft(persist = true)
                    }
                }
            }.fold(
                onSuccess = { _events.emit(EditorEvent.Closed) },
                onFailure = { throwable ->
                    isClosing = false
                    _uiState.update {
                        it.copy(errorMessage = throwable.message ?: "Failed to persist draft")
                    }
                },
            )
        }
    }

    private suspend fun synchronizeCurrentDraft(persist: Boolean): NoteDraftRecord? {
        val draft = currentDraft ?: return null
        val state = _uiState.value
        val updated = repository.updateDraft(
            draftId = draft.id,
            content = state.content,
            tags = state.tags,
            color = state.noteColorCss,
            updateColor = true,
            expectedRevision = draft.revision.takeIf { draft.persisted },
        )
        val synchronized = if (persist && !updated.persisted) {
            repository.persistDraft(updated.id)
        } else {
            updated
        }
        currentDraft = synchronized
        return synchronized
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
        private const val AUTO_SAVE_DEBOUNCE_MS = 800L
    }
}
