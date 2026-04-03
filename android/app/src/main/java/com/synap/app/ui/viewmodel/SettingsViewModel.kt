package com.synap.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.getBuildInfo
import com.synap.app.di.IoDispatcher
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class SettingsUiState(
    val buildVersion: String = "加载中...",
    val buildVersionDetails: String? = null,
)

@HiltViewModel
class SettingsViewModel @Inject constructor(
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
) : ViewModel() {
    private val _uiState = MutableStateFlow(SettingsUiState())
    val uiState: StateFlow<SettingsUiState> = _uiState.asStateFlow()

    init {
        loadBuildInfo()
    }

    private fun loadBuildInfo() {
        viewModelScope.launch(ioDispatcher) {
            val buildInfo = runCatching { getBuildInfo() }.getOrNull()
            _uiState.value = SettingsUiState(
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
