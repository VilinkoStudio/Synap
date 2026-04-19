package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.getBuildInfo
import com.synap.app.data.repository.SyncRepository
import com.synap.app.di.IoDispatcher
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class SettingsUiState(
    val buildVersion: String = "加载中...",
    val buildVersionDetails: String? = null,
    val syncStatus: String = "同步监听未启动",
    val syncPort: Int? = null,
    val syncAddresses: List<String> = emptyList(),
)

@HiltViewModel
class SettingsViewModel @Inject constructor(
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
    private val syncRepository: SyncRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(SettingsUiState())
    val uiState: StateFlow<SettingsUiState> = _uiState.asStateFlow()

    init {
        loadBuildInfo()
        observeSyncRuntime()
    }

    private fun loadBuildInfo() {
        viewModelScope.launch(ioDispatcher) {
            val buildInfo = runCatching { getBuildInfo() }.getOrNull()
            _uiState.update {
                it.copy(
                    buildVersion = buildInfo?.displayVersion ?: "读取失败",
                    buildVersionDetails = buildInfo?.let { info ->
                        buildString {
                            append(info.gitBranch)
                            append(" @ ")
                            append(info.gitShortCommit)
                        }
                    },
                )
            }
        }
    }

    private fun observeSyncRuntime() {
        viewModelScope.launch {
            syncRepository.runtimeState.collect { state ->
                _uiState.update {
                    it.copy(
                        syncStatus = when {
                            state.listenPort != null -> "${state.protocol} ${state.status}"
                            state.errorMessage != null -> "${state.status} · ${state.errorMessage}"
                            else -> state.status
                        },
                        syncPort = state.listenPort,
                        syncAddresses = state.localAddresses,
                    )
                }
            }
        }
    }
}
