package com.synap.app.data.repository

import com.synap.app.data.model.StarmapPointRecord
import com.synap.app.data.service.SynapServiceApi
import javax.inject.Inject
import javax.inject.Singleton

interface StarmapRepository {
    suspend fun getStarmap(): List<StarmapPointRecord>
}

@Singleton
class StarmapRepositoryImpl @Inject constructor(
    private val service: SynapServiceApi,
) : StarmapRepository {
    override suspend fun getStarmap(): List<StarmapPointRecord> {
        if (!service.isInitialized) {
            service.initialize().unwrap()
        }
        return service.getStarmap().unwrap()
    }

    private fun <T> Result<T>.unwrap(): T = getOrElse { throw it }
}
