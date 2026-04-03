package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.repository.SynapRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface AppSessionUiState {
    data object Initializing : AppSessionUiState
    data object Ready : AppSessionUiState
    data class Error(val message: String) : AppSessionUiState
}

@HiltViewModel
class AppSessionViewModel @Inject constructor(
    private val repository: SynapRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow<AppSessionUiState>(AppSessionUiState.Initializing)
    val uiState: StateFlow<AppSessionUiState> = _uiState.asStateFlow()

    init {
        initialize()
    }

    fun initialize() {
        viewModelScope.launch {
            _uiState.value = AppSessionUiState.Initializing
            runCatching {
                repository.initialize()
            }.fold(
                onSuccess = { _uiState.value = AppSessionUiState.Ready },
                onFailure = { throwable ->
                    _uiState.value = AppSessionUiState.Error(
                        throwable.message ?: "Failed to initialize Synap",
                    )
                },
            )
        }
    }
}
