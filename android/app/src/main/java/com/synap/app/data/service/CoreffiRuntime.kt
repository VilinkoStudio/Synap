package com.synap.app.data.service

import android.content.Context
import android.util.Log
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.FfiException
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SynapService as FfiSynapService
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.open
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.openMemory
import com.synap.app.data.error.SynapError
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.toNoteRecord
import com.synap.app.data.model.toNoteRecords
import com.synap.app.di.IoDispatcher
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

private const val TAG = "CoreffiRuntime"

@Singleton
class CoreffiRuntime @Inject constructor(
    @ApplicationContext private val context: Context,
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
) : SynapServiceApi {
    private val mutex = Mutex()
    private var serviceInstance: FfiSynapService? = null

    override val isInitialized: Boolean
        get() = serviceInstance != null

    override suspend fun initialize(): Result<Unit> = mutex.withLock {
        withContext(ioDispatcher) {
            runCatching {
                serviceInstance?.close()
                val dbFile = databaseFile()
                dbFile.parentFile?.mkdirs()
                serviceInstance = open(dbFile.absolutePath)
            }.fold(
                onSuccess = { Result.success(Unit) },
                onFailure = { throwable ->
                    Log.e(TAG, "Failed to initialize coreffi runtime", throwable)
                    Result.failure(throwable.toSynapError())
                },
            )
        }
    }

    override suspend fun initializeInMemory(): Result<Unit> = mutex.withLock {
        withContext(ioDispatcher) {
            runCatching {
                serviceInstance?.close()
                serviceInstance = openMemory()
            }.fold(
                onSuccess = { Result.success(Unit) },
                onFailure = { throwable -> Result.failure(throwable.toSynapError()) },
            )
        }
    }

    override suspend fun close(): Result<Unit> = mutex.withLock {
        withContext(ioDispatcher) {
            runCatching {
                serviceInstance?.close()
                serviceInstance = null
            }.fold(
                onSuccess = { Result.success(Unit) },
                onFailure = { throwable -> Result.failure(throwable.toSynapError()) },
            )
        }
    }

    override suspend fun exportDatabase(outputStream: OutputStream): Result<Unit> = mutex.withLock {
        withContext(ioDispatcher) {
            runCatching {
                val dbFile = databaseFile()
                if (!dbFile.exists()) {
                    throw SynapError.Database(message = "Database file does not exist.")
                }

                val shouldReopen = serviceInstance != null
                if (shouldReopen) {
                    serviceInstance?.close()
                    serviceInstance = null
                }

                try {
                    dbFile.inputStream().buffered().use { input ->
                        outputStream.buffered().use { output ->
                            input.copyTo(output)
                            output.flush()
                        }
                    }
                } finally {
                    if (shouldReopen) {
                        serviceInstance = open(dbFile.absolutePath)
                    }
                }
            }.fold(
                onSuccess = { Result.success(Unit) },
                onFailure = { throwable ->
                    Log.e(TAG, "Failed to export database", throwable)
                    Result.failure(throwable.toSynapError())
                },
            )
        }
    }

    override suspend fun replaceDatabase(inputStream: InputStream): Result<Unit> = mutex.withLock {
        withContext(ioDispatcher) {
            runCatching {
                val dbFile = databaseFile()
                dbFile.parentFile?.mkdirs()

                val tempFile = File(dbFile.parentFile, "${dbFile.name}.importing")
                tempFile.delete()

                val shouldRestoreOnFailure = serviceInstance != null
                serviceInstance?.close()
                serviceInstance = null

                try {
                    tempFile.outputStream().buffered().use { output ->
                        inputStream.buffered().use { input ->
                            input.copyTo(output)
                            output.flush()
                        }
                    }
                    tempFile.copyTo(dbFile, overwrite = true)
                    tempFile.delete()
                } catch (throwable: Throwable) {
                    tempFile.delete()
                    if (shouldRestoreOnFailure && dbFile.exists()) {
                        runCatching {
                            serviceInstance = open(dbFile.absolutePath)
                        }.onFailure { restoreError ->
                            throwable.addSuppressed(restoreError)
                        }
                    }
                    throw throwable
                }
            }.fold(
                onSuccess = { Result.success(Unit) },
                onFailure = { throwable ->
                    Log.e(TAG, "Failed to replace database", throwable)
                    Result.failure(throwable.toSynapError())
                },
            )
        }
    }

    override suspend fun getNote(idOrShortId: String): Result<NoteRecord> =
        withService { service -> service.getNote(idOrShortId).toNoteRecord() }

    override suspend fun getReplies(
        parentId: String,
        cursor: String?,
        limit: UInt,
    ): Result<List<NoteRecord>> =
        withService { service -> service.getReplies(parentId, cursor, limit).toNoteRecords() }

    override suspend fun getRecentNote(cursor: String?, limit: UInt?): Result<List<NoteRecord>> =
        withService { service -> service.getRecentNote(cursor, limit).toNoteRecords() }

    override suspend fun getOrigins(childId: String): Result<List<NoteRecord>> =
        withService { service -> service.getOrigins(childId).toNoteRecords() }

    override suspend fun getPreviousVersions(noteId: String): Result<List<NoteRecord>> =
        withService { service -> service.getPreviousVersions(noteId).toNoteRecords() }

    override suspend fun getNextVersions(noteId: String): Result<List<NoteRecord>> =
        withService { service -> service.getNextVersions(noteId).toNoteRecords() }

    override suspend fun getOtherVersions(noteId: String): Result<List<NoteRecord>> =
        withService { service -> service.getOtherVersions(noteId).toNoteRecords() }

    override suspend fun getDeletedNotes(cursor: String?, limit: UInt?): Result<List<NoteRecord>> =
        withService { service -> service.getDeletedNotes(cursor, limit).toNoteRecords() }

    override suspend fun search(query: String, limit: UInt): Result<List<NoteRecord>> =
        withService { service -> service.search(query, limit).toNoteRecords() }

    override suspend fun searchTags(query: String, limit: UInt): Result<List<String>> =
        withService { service -> service.searchTags(query, limit) }

    override suspend fun createNote(content: String, tags: List<String>): Result<NoteRecord> =
        withService { service -> service.createNote(content, tags).toNoteRecord() }

    override suspend fun replyNote(
        parentId: String,
        content: String,
        tags: List<String>,
    ): Result<NoteRecord> =
        withService { service -> service.replyNote(parentId, content, tags).toNoteRecord() }

    override suspend fun editNote(
        targetId: String,
        newContent: String,
        tags: List<String>,
    ): Result<NoteRecord> =
        withService { service -> service.editNote(targetId, newContent, tags).toNoteRecord() }

    override suspend fun deleteNote(targetId: String): Result<Unit> =
        withService { service -> service.deleteNote(targetId) }

    override suspend fun restoreNote(targetId: String): Result<Unit> =
        withService { service -> service.restoreNote(targetId) }

    private suspend fun <T> withService(block: (FfiSynapService) -> T): Result<T> =
        mutex.withLock {
            withContext(ioDispatcher) {
                runCatching {
                    val service = serviceInstance ?: throw SynapError.Database(
                        message = "Service not initialized. Call initialize() first.",
                    )
                    block(service)
                }.fold(
                    onSuccess = { Result.success(it) },
                    onFailure = { throwable ->
                        Log.e(TAG, "Coreffi call failed", throwable)
                        Result.failure(throwable.toSynapError())
                    },
                )
            }
        }

    private fun databaseFile(): File = context.getDatabasePath("synap.redb")

    private fun Throwable.toSynapError(): SynapError = when (this) {
        is SynapError -> this
        is FfiException -> SynapError.fromFfiException(this)
        else -> SynapError.Unknown(
            message = message ?: javaClass.simpleName,
            cause = this,
        )
    }
}
