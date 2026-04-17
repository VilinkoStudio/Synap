package com.synap.app.ui.viewmodel

import android.net.Uri
import android.util.Base64
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.synap.app.data.model.ShareImportStats
import com.synap.app.data.repository.SynapRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface ShareImportUiState {
    data object Idle : ShareImportUiState
    data object Importing : ShareImportUiState
    data class Success(val stats: ShareImportStats) : ShareImportUiState
    data class Error(val message: String) : ShareImportUiState
}

@HiltViewModel
class ShareImportViewModel @Inject constructor(
    private val repository: SynapRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow<ShareImportUiState>(ShareImportUiState.Idle)
    val uiState: StateFlow<ShareImportUiState> = _uiState.asStateFlow()

    fun importFromDeepLink(uri: Uri) {
        viewModelScope.launch {
            _uiState.value = ShareImportUiState.Importing

            runCatching {
                val encodedPayload = extractEncodedPayload(uri)
                val bytes = Base64.decode(
                    encodedPayload,
                    Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING,
                )
                repository.importShare(bytes)
            }.fold(
                onSuccess = { stats ->
                    _uiState.value = ShareImportUiState.Success(stats)
                },
                onFailure = { throwable ->
                    _uiState.value = ShareImportUiState.Error(
                        throwable.message ?: "导入分享内容失败",
                    )
                },
            )
        }
    }

    fun clearState() {
        _uiState.value = ShareImportUiState.Idle
    }

    private fun extractEncodedPayload(uri: Uri): String {
        val pathPayload = uri.pathSegments
            .firstOrNull()
            ?.takeIf { it.isNotBlank() }
        val queryPayload = uri.getQueryParameter("payload")
            ?.takeIf { it.isNotBlank() }

        return pathPayload
            ?: queryPayload
            ?: throw IllegalArgumentException("分享链接缺少导入内容")
    }
}
