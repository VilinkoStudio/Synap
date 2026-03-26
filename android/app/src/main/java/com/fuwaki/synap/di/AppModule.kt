package com.fuwaki.synap.di

import com.fuwaki.synap.data.repository.SynapRepository
import com.fuwaki.synap.data.repository.SynapRepositoryImpl
import com.fuwaki.synap.data.service.CoreffiRuntime
import com.fuwaki.synap.data.service.SynapServiceApi
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
}
