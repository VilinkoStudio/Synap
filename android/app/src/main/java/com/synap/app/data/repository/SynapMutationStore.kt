package com.synap.app.data.repository

import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

@Singleton
class SynapMutationStore @Inject constructor() {
    private val _mutations = MutableSharedFlow<SynapMutation>(extraBufferCapacity = 32)
    val mutations: SharedFlow<SynapMutation> = _mutations.asSharedFlow()

    fun emit(mutation: SynapMutation) {
        _mutations.tryEmit(mutation)
    }
}
