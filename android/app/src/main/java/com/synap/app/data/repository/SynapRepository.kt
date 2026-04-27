package com.synap.app.data.repository

import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.NoteVersionRecord
import com.synap.app.data.model.ReplyItem
import com.synap.app.data.model.SearchResultRecord
import com.synap.app.data.model.ShareImportStats
import com.synap.app.data.model.TimelineDirection
import com.synap.app.data.model.TimelineSessionRecord
import com.synap.app.data.portal.CursorPortal
import com.synap.app.data.service.SynapServiceApi
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.SharedFlow

sealed interface SynapMutation {
    data class Created(val noteId: String) : SynapMutation
    data class Replied(val parentId: String, val noteId: String) : SynapMutation
    data class Edited(val oldId: String, val newId: String) : SynapMutation
    data class Deleted(val targetId: String) : SynapMutation
    data class Restored(val targetId: String) : SynapMutation
    data class Imported(val stats: ShareImportStats) : SynapMutation
}

interface SynapRepository {
    val mutations: SharedFlow<SynapMutation>

    suspend fun initialize()

    suspend fun shutdown()

    fun openRecentPortal(limit: UInt = 20u): CursorPortal<NoteRecord>

    fun openRecentSessionsPortal(limit: UInt = 20u): CursorPortal<TimelineSessionRecord>

    fun openRepliesPortal(parentId: String, limit: UInt = 20u): CursorPortal<ReplyItem>

    fun openDeletedPortal(limit: UInt = 20u): CursorPortal<NoteRecord>

    fun openTaggedPortal(tag: String, limit: UInt = 20u): CursorPortal<NoteRecord>

    fun openFilteredPortal(
        filter: NoteFeedFilter,
        limit: UInt = 20u,
    ): CursorPortal<NoteRecord>

    suspend fun getNote(idOrShortId: String): NoteRecord

    suspend fun getOrigins(noteId: String): List<NoteRecord>

    suspend fun getPreviousVersions(noteId: String): List<NoteVersionRecord>

    suspend fun getNextVersions(noteId: String): List<NoteVersionRecord>

    suspend fun getOtherVersions(noteId: String): List<NoteVersionRecord>

    suspend fun search(query: String, limit: UInt = 50u): List<NoteRecord>

    suspend fun searchFusion(
        query: String,
        limit: UInt = 50u,
        fuzzyLimit: UInt? = null,
        semanticLimit: UInt? = null,
    ): List<SearchResultRecord>

    suspend fun searchTags(query: String, limit: UInt = 10u): List<String>

    suspend fun recommendTag(content: String, limit: UInt = 6u): List<String>

    suspend fun getAllTags(): List<String>

    suspend fun getNotesByTag(tag: String, cursor: String?, limit: UInt? = 20u): List<NoteRecord>

    suspend fun getFilteredNotes(
        filter: NoteFeedFilter,
        cursor: String?,
        limit: UInt? = 20u,
    ): List<NoteRecord>

    suspend fun exportShare(noteIds: List<String>): ByteArray

    suspend fun importShare(bytes: ByteArray): ShareImportStats

    suspend fun createNote(content: String, tags: List<String>): NoteRecord

    suspend fun replyToNote(parentId: String, content: String, tags: List<String>): NoteRecord

    suspend fun editNote(targetId: String, newContent: String, tags: List<String>): NoteRecord

    suspend fun deleteNote(targetId: String)

    suspend fun restoreNote(targetId: String)
}

@Singleton
class SynapRepositoryImpl @Inject constructor(
    private val service: SynapServiceApi,
    private val mutationStore: SynapMutationStore,
) : SynapRepository {
    override val mutations: SharedFlow<SynapMutation> = mutationStore.mutations

    override suspend fun initialize() {
        service.initialize().unwrap()
    }

    override suspend fun shutdown() {
        service.close().unwrap()
    }

    override fun openRecentPortal(limit: UInt): CursorPortal<NoteRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getRecentNotesPage(
                    cursor = cursor,
                    direction = TimelineDirection.Older,
                    limit = pageLimit.takeIf { it > 0u },
                ).unwrap()
            },
        )

    override fun openRecentSessionsPortal(limit: UInt): CursorPortal<TimelineSessionRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getRecentSessionsPage(
                    cursor = cursor,
                    limit = pageLimit.takeIf { it > 0u },
                ).unwrap()
            },
        )

    override fun openRepliesPortal(parentId: String, limit: UInt): CursorPortal<ReplyItem> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getReplies(parentId, cursor, pageLimit)
                    .unwrap()
                    .map { ReplyItem(note = it, parentId = parentId) }
            },
            cursorOf = { item -> item.note.id },
        )

    override fun openDeletedPortal(limit: UInt): CursorPortal<NoteRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getDeletedNotes(cursor, pageLimit.takeIf { it > 0u }).unwrap()
            },
            cursorOf = NoteRecord::id,
        )

    override fun openTaggedPortal(tag: String, limit: UInt): CursorPortal<NoteRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getNotesByTag(tag, cursor, pageLimit.takeIf { it > 0u }).unwrap()
            },
            cursorOf = NoteRecord::id,
        )

    override fun openFilteredPortal(filter: NoteFeedFilter, limit: UInt): CursorPortal<NoteRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getFilteredNotesPage(
                    filter = filter,
                    cursor = cursor,
                    direction = TimelineDirection.Older,
                    limit = pageLimit.takeIf { it > 0u },
                ).unwrap()
            },
        )

    override suspend fun getNote(idOrShortId: String): NoteRecord =
        service.getNote(idOrShortId).unwrap()

    override suspend fun getOrigins(noteId: String): List<NoteRecord> =
        service.getOrigins(noteId).unwrap()

    override suspend fun getPreviousVersions(noteId: String): List<NoteVersionRecord> =
        service.getPreviousVersions(noteId).unwrap()

    override suspend fun getNextVersions(noteId: String): List<NoteVersionRecord> =
        service.getNextVersions(noteId).unwrap()

    override suspend fun getOtherVersions(noteId: String): List<NoteVersionRecord> =
        service.getOtherVersions(noteId).unwrap()

    override suspend fun search(query: String, limit: UInt): List<NoteRecord> =
        service.search(query, limit).unwrap()

    override suspend fun searchFusion(
        query: String,
        limit: UInt,
        fuzzyLimit: UInt?,
        semanticLimit: UInt?,
    ): List<SearchResultRecord> =
        service.searchFusion(query, limit, fuzzyLimit, semanticLimit).unwrap()

    override suspend fun searchTags(query: String, limit: UInt): List<String> =
        service.searchTags(query, limit).unwrap()

    override suspend fun recommendTag(content: String, limit: UInt): List<String> =
        service.recommendTag(content, limit).unwrap()

    override suspend fun getAllTags(): List<String> =
        service.getAllTags().unwrap()

    override suspend fun getNotesByTag(tag: String, cursor: String?, limit: UInt?): List<NoteRecord> =
        service.getNotesByTag(tag, cursor, limit).unwrap()

    override suspend fun getFilteredNotes(
        filter: NoteFeedFilter,
        cursor: String?,
        limit: UInt?,
    ): List<NoteRecord> = service.getFilteredNotes(filter, cursor, limit).unwrap()

    override suspend fun exportShare(noteIds: List<String>): ByteArray =
        service.exportShare(noteIds).unwrap()

    override suspend fun importShare(bytes: ByteArray): ShareImportStats {
        if (!service.isInitialized) {
            service.initialize().unwrap()
        }
        val stats = service.importShare(bytes).unwrap()
        mutationStore.emit(SynapMutation.Imported(stats))
        return stats
    }

    override suspend fun createNote(content: String, tags: List<String>): NoteRecord {
        val created = service.createNote(content, tags).unwrap()
        mutationStore.emit(SynapMutation.Created(created.id))
        return created
    }

    override suspend fun replyToNote(parentId: String, content: String, tags: List<String>): NoteRecord {
        val created = service.replyNote(parentId, content, tags).unwrap()
        mutationStore.emit(SynapMutation.Replied(parentId = parentId, noteId = created.id))
        return created
    }

    override suspend fun editNote(targetId: String, newContent: String, tags: List<String>): NoteRecord {
        val edited = service.editNote(targetId, newContent, tags).unwrap()
        mutationStore.emit(SynapMutation.Edited(oldId = targetId, newId = edited.id))
        return edited
    }

    override suspend fun deleteNote(targetId: String) {
        service.deleteNote(targetId).unwrap()
        mutationStore.emit(SynapMutation.Deleted(targetId))
    }

    override suspend fun restoreNote(targetId: String) {
        service.restoreNote(targetId).unwrap()
        mutationStore.emit(SynapMutation.Restored(targetId))
    }

    private fun <T> Result<T>.unwrap(): T = getOrElse { throw it }
}
