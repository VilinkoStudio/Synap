package com.synap.app.data.service

import android.content.Context
import android.util.Log
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.FfiException
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.FilteredNoteStatus as FfiFilteredNoteStatus
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SynapService as FfiSynapService
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SyncTransport as FfiSyncTransport
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.TimelineDirection as FfiTimelineDirection
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.open
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.openMemory
import com.synap.app.data.error.SynapError
import com.synap.app.data.model.LocalIdentity
import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.NoteFeedStatus
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.NoteVersionRecord
import com.synap.app.data.model.PeerRecord
import com.synap.app.data.model.SearchResultRecord
import com.synap.app.data.model.ShareImportStats
import com.synap.app.data.model.StarmapPointRecord
import com.synap.app.data.model.SyncSession
import com.synap.app.data.model.SyncSessionRecord
import com.synap.app.data.model.TimelineDirection
import com.synap.app.data.model.TimelineSessionRecord
import com.synap.app.data.model.toLocalIdentity
import com.synap.app.data.model.toPeerRecord
import com.synap.app.data.model.toPeerRecords
import com.synap.app.data.model.toSyncSession
import com.synap.app.data.model.toSyncSessionRecords
import com.synap.app.data.model.toShareImportStats
import com.synap.app.data.model.toStarmapPoints
import com.synap.app.data.model.toCursorPage
import com.synap.app.data.model.toDto
import com.synap.app.data.model.toNoteRecord
import com.synap.app.data.model.toNoteRecords
import com.synap.app.data.model.toNoteVersionRecords
import com.synap.app.data.model.toSearchResultRecords
import com.synap.app.data.model.toCursorPage as toSessionCursorPage
import com.synap.app.data.portal.CursorPage
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
    @Volatile
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

    override suspend fun exportShare(noteIds: List<String>): Result<ByteArray> =
        withService { service -> service.exportShare(noteIds) }

    override suspend fun importShare(bytes: ByteArray): Result<ShareImportStats> =
        withService { service ->
            service.importShare(bytes).toShareImportStats()
        }

    override suspend fun getLocalIdentity(): Result<LocalIdentity> =
        withService { service -> service.getLocalIdentity().toLocalIdentity() }

    override suspend fun getPeers(): Result<List<PeerRecord>> =
        withService { service -> service.getPeers().toPeerRecords() }

    override suspend fun trustPeer(publicKey: ByteArray, note: String?): Result<PeerRecord> =
        withService { service -> service.trustPeer(publicKey, note).toPeerRecord() }

    override suspend fun updatePeerNote(peerId: String, note: String?): Result<PeerRecord> =
        withService { service -> service.updatePeerNote(peerId, note).toPeerRecord() }

    override suspend fun setPeerStatus(
        peerId: String,
        status: com.synap.app.data.model.PeerTrustStatus,
    ): Result<PeerRecord> =
        withService { service -> service.setPeerStatus(peerId, status.toDto()).toPeerRecord() }

    override suspend fun deletePeer(peerId: String): Result<Unit> =
        withService { service ->
            service.deletePeer(peerId)
            Unit
        }

    override suspend fun getRecentSyncSessions(limit: UInt?): Result<List<SyncSessionRecord>> =
        withService { service ->
            service.getRecentSyncSessions(limit).toSyncSessionRecords()
        }

    override suspend fun initiateSync(transport: SyncTransportChannel): Result<SyncSession> =
        withService { service ->
            service.initiateSync(FfiSyncTransportAdapter(transport)).toSyncSession()
        }

    override suspend fun listenSync(transport: SyncTransportChannel): Result<SyncSession> =
        withService { service ->
            service.listenSync(FfiSyncTransportAdapter(transport)).toSyncSession()
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

    override suspend fun getRecentNotesPage(
        cursor: String?,
        direction: TimelineDirection,
        limit: UInt?,
    ): Result<CursorPage<NoteRecord>> =
        withService { service ->
            service.getRecentNotesPage(cursor, direction.toFfiDirection(), limit).toCursorPage()
        }

    override suspend fun getRecentSessionsPage(
        cursor: String?,
        limit: UInt?,
    ): Result<CursorPage<TimelineSessionRecord>> =
        withService { service ->
            service.getRecentSessionsPage(cursor, limit).toSessionCursorPage()
        }

    override suspend fun getOrigins(childId: String): Result<List<NoteRecord>> =
        withService { service -> service.getOrigins(childId).toNoteRecords() }

    override suspend fun getPreviousVersions(noteId: String): Result<List<NoteVersionRecord>> =
        withService { service -> service.getPreviousVersions(noteId).toNoteVersionRecords() }

    override suspend fun getNextVersions(noteId: String): Result<List<NoteVersionRecord>> =
        withService { service -> service.getNextVersions(noteId).toNoteVersionRecords() }

    override suspend fun getOtherVersions(noteId: String): Result<List<NoteVersionRecord>> =
        withService { service -> service.getOtherVersions(noteId).toNoteVersionRecords() }

    override suspend fun getDeletedNotes(cursor: String?, limit: UInt?): Result<List<NoteRecord>> =
        withService { service -> service.getDeletedNotes(cursor, limit).toNoteRecords() }

    override suspend fun getStarmap(): Result<List<StarmapPointRecord>> =
        withService { service -> service.getStarmap().toStarmapPoints() }

    override suspend fun search(query: String, limit: UInt): Result<List<NoteRecord>> =
        withService { service -> service.search(query, limit).toNoteRecords() }

    override suspend fun searchFusion(
        query: String,
        limit: UInt,
        fuzzyLimit: UInt?,
        semanticLimit: UInt?,
    ): Result<List<SearchResultRecord>> =
        withService { service ->
            service
                .searchFusion(query, limit, fuzzyLimit, semanticLimit)
                .toSearchResultRecords()
        }

    override suspend fun searchTags(query: String, limit: UInt): Result<List<String>> =
        withService { service -> service.searchTags(query, limit) }

    override suspend fun recommendTag(content: String, limit: UInt): Result<List<String>> =
        withService { service -> service.recommendTag(content, limit) }

    override suspend fun getAllTags(): Result<List<String>> =
        withService { service -> service.getAllTags() }

    override suspend fun getNotesByTag(
        tag: String,
        cursor: String?,
        limit: UInt?,
    ): Result<List<NoteRecord>> =
        withService { service -> service.getNotesByTag(tag, cursor, limit).toNoteRecords() }

    override suspend fun getFilteredNotes(
        filter: NoteFeedFilter,
        cursor: String?,
        limit: UInt?,
    ): Result<List<NoteRecord>> =
        withService { service ->
            service.getFilteredNotes(
                filter.selectedTags,
                filter.includeUntagged,
                filter.tagFilterEnabled,
                filter.status.toFfiStatus(),
                cursor,
                limit,
            ).toNoteRecords()
        }

    override suspend fun getFilteredNotesPage(
        filter: NoteFeedFilter,
        cursor: String?,
        direction: TimelineDirection,
        limit: UInt?,
    ): Result<CursorPage<NoteRecord>> =
        withService { service ->
            service.getFilteredNotesPage(
                filter.selectedTags,
                filter.includeUntagged,
                filter.tagFilterEnabled,
                filter.status.toFfiStatus(),
                cursor,
                direction.toFfiDirection(),
                limit,
            ).toCursorPage()
        }

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

    private fun databaseFile(): File = context.getDatabasePath("synap.redb")

    private fun NoteFeedStatus.toFfiStatus(): FfiFilteredNoteStatus = when (this) {
        NoteFeedStatus.All -> FfiFilteredNoteStatus.ALL
        NoteFeedStatus.Normal -> FfiFilteredNoteStatus.NORMAL
        NoteFeedStatus.Deleted -> FfiFilteredNoteStatus.DELETED
    }

    private fun TimelineDirection.toFfiDirection(): FfiTimelineDirection = when (this) {
        TimelineDirection.Older -> FfiTimelineDirection.OLDER
        TimelineDirection.Newer -> FfiTimelineDirection.NEWER
    }

    private fun Throwable.toSynapError(): SynapError = when (this) {
        is SynapError -> this
        is FfiException -> SynapError.fromFfiException(this)
        else -> SynapError.Unknown(
            message = message ?: javaClass.simpleName,
            cause = this,
        )
    }
}

private class FfiSyncTransportAdapter(
    private val transport: SyncTransportChannel,
) : FfiSyncTransport {
    override fun read(maxBytes: UInt): ByteArray =
        kotlinx.coroutines.runBlocking {
            transport.read(maxBytes.toInt())
        }

    override fun write(payload: ByteArray) {
        kotlinx.coroutines.runBlocking {
            transport.write(payload)
        }
    }

    override fun close() {
        kotlinx.coroutines.runBlocking {
            transport.close()
        }
    }
}
