package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.model.StarmapPointRecord
import com.synap.app.data.repository.SynapRepository
import com.synap.app.data.repository.StarmapRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class StarmapNoteSnapshot(
    val content: String,
    val createdAt: Long,
)

data class StarmapUiState(
    val points: List<StarmapPointRecord> = emptyList(),
    val noteSnapshots: Map<String, StarmapNoteSnapshot> = emptyMap(),
    val isLoading: Boolean = true,
    val errorMessage: String? = null,
)

@HiltViewModel
class StarmapViewModel @Inject constructor(
    private val repository: StarmapRepository,
    private val synapRepository: SynapRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(StarmapUiState())
    val uiState: StateFlow<StarmapUiState> = _uiState.asStateFlow()

    init {
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(
                isLoading = true,
                errorMessage = null,
            )

            runCatching { repository.getStarmap() }
                .onSuccess { points ->
                    _uiState.value = StarmapUiState(
                        points = points,
                        noteSnapshots = emptyMap(),
                        isLoading = false,
                        errorMessage = null,
                    )
                    loadNoteContents(points)
                }
                .onFailure { throwable ->
                    _uiState.value = _uiState.value.copy(
                        isLoading = false,
                        errorMessage = throwable.message ?: "Failed to load starmap",
                    )
                }
        }
    }

    private fun loadNoteContents(points: List<StarmapPointRecord>) {
        viewModelScope.launch {
            val snapshots = runCatching {
                coroutineScope {
                    points.map { point ->
                        async {
                            val note = synapRepository.getNote(point.id)
                            point.id to StarmapNoteSnapshot(
                                content = note.content,
                                createdAt = note.createdAt,
                            )
                        }
                    }.awaitAll().toMap()
                }
            }.getOrElse { emptyMap() }

            _uiState.value = _uiState.value.copy(
                noteSnapshots = snapshots,
            )
        }
    }
}
