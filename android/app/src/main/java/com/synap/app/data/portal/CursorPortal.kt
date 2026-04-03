package com.synap.app.data.portal

import com.synap.app.data.error.SynapError
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

data class PortalState<T>(
    val items: List<T> = emptyList(),
    val isLoading: Boolean = false,
    val hasMore: Boolean = true,
    val isStale: Boolean = true,
    val error: SynapError? = null,
)

class CursorPortal<T>(
    private val limit: UInt,
    private val fetchPage: suspend (cursor: String?, limit: UInt) -> List<T>,
    private val cursorOf: (T) -> String,
) {
    private val mutex = Mutex()
    private var cursor: String? = null
    private var seeded = false
    private val _state = MutableStateFlow(PortalState<T>())
    val state: StateFlow<PortalState<T>> = _state.asStateFlow()

    suspend fun refresh() {
        mutex.withLock {
            loadLocked(reset = true)
        }
    }

    suspend fun loadNext() {
        mutex.withLock {
            if (!seeded || _state.value.isStale) {
                loadLocked(reset = true)
            } else if (_state.value.hasMore && !_state.value.isLoading) {
                loadLocked(reset = false)
            }
        }
    }

    fun invalidate() {
        _state.update { it.copy(isStale = true) }
    }

    private suspend fun loadLocked(reset: Boolean) {
        val current = _state.value
        _state.update { it.copy(isLoading = true, error = null) }
        val requestCursor = if (reset) null else cursor

        try {
            val page = fetchPage(requestCursor, limit)
            val mergedItems = if (reset) page else current.items + page
            cursor = page.lastOrNull()?.let(cursorOf)
            seeded = true

            _state.value = PortalState(
                items = mergedItems,
                isLoading = false,
                hasMore = page.isNotEmpty() && page.size == limit.toInt(),
                isStale = false,
                error = null,
            )
        } catch (throwable: Throwable) {
            val error = throwable as? SynapError
                ?: SynapError.Unknown(
                    message = throwable.message ?: "An unknown error occurred",
                    cause = throwable,
                )
            if (reset) {
                _state.value = PortalState(
                    items = emptyList(),
                    isLoading = false,
                    hasMore = true,
                    isStale = true,
                    error = error,
                )
            } else {
                _state.update { it.copy(isLoading = false, error = error) }
            }
        }
    }
}
