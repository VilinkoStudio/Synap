package com.synap.app.di

import com.synap.app.data.repository.SynapRepository
import com.synap.app.data.repository.SynapRepositoryImpl
import com.synap.app.data.repository.SyncRepository
import com.synap.app.data.repository.SyncRepositoryImpl
import com.synap.app.data.service.CoreffiRuntime
import com.synap.app.data.service.SynapServiceApi
import com.synap.app.data.service.AndroidSyncDiscoveryRuntime
import com.synap.app.data.service.SyncDiscoveryRuntime
import com.synap.app.data.service.SyncNetworkRuntime
import com.synap.app.data.service.TcpSyncNetworkRuntime
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers

@Module
@InstallIn(SingletonComponent::class)
object AppModule {
    @Provides
    @IoDispatcher
    fun provideIoDispatcher(): CoroutineDispatcher = Dispatchers.IO

    @Provides
    @Singleton
    fun provideSynapServiceApi(runtime: CoreffiRuntime): SynapServiceApi = runtime

    @Provides
    @Singleton
    fun provideSynapRepository(impl: SynapRepositoryImpl): SynapRepository = impl

    @Provides
    @Singleton
    fun provideSyncNetworkRuntime(runtime: TcpSyncNetworkRuntime): SyncNetworkRuntime = runtime

    @Provides
    @Singleton
    fun provideSyncDiscoveryRuntime(runtime: AndroidSyncDiscoveryRuntime): SyncDiscoveryRuntime = runtime

    @Provides
    @Singleton
    fun provideSyncRepository(impl: SyncRepositoryImpl): SyncRepository = impl
}
