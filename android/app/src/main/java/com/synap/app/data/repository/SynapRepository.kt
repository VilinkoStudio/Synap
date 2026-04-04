package com.synap.app.data.repository

import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.ReplyItem
import com.synap.app.data.model.TimelineDirection
import com.synap.app.data.model.TimelineSessionRecord
import com.synap.app.data.portal.CursorPortal
import com.synap.app.data.service.SynapServiceApi
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

sealed interface SynapMutation {
    data class Created(val noteId: String) : SynapMutation
    data class Replied(val parentId: String, val noteId: String) : SynapMutation
    data class Edited(val oldId: String, val newId: String) : SynapMutation
    data class Deleted(val targetId: String) : SynapMutation
    data class Restored(val targetId: String) : SynapMutation
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

    suspend fun getPreviousVersions(noteId: String): List<NoteRecord>

    suspend fun getNextVersions(noteId: String): List<NoteRecord>

    suspend fun getOtherVersions(noteId: String): List<NoteRecord>

    suspend fun search(query: String, limit: UInt = 50u): List<NoteRecord>

    suspend fun searchTags(query: String, limit: UInt = 10u): List<String>

    suspend fun getAllTags(): List<String>

    suspend fun getNotesByTag(tag: String, cursor: String?, limit: UInt? = 20u): List<NoteRecord>

    suspend fun getFilteredNotes(
        filter: NoteFeedFilter,
        cursor: String?,
        limit: UInt? = 20u,
    ): List<NoteRecord>

    suspend fun createNote(content: String, tags: List<String>): NoteRecord

    suspend fun replyToNote(parentId: String, content: String, tags: List<String>): NoteRecord

    suspend fun editNote(targetId: String, newContent: String, tags: List<String>): NoteRecord

    suspend fun deleteNote(targetId: String)

    suspend fun restoreNote(targetId: String)
}

@Singleton
class SynapRepositoryImpl @Inject constructor(
    private val service: SynapServiceApi,
) : SynapRepository {
    private val _mutations = MutableSharedFlow<SynapMutation>(extraBufferCapacity = 32)
    override val mutations: SharedFlow<SynapMutation> = _mutations.asSharedFlow()

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

    override suspend fun getPreviousVersions(noteId: String): List<NoteRecord> =
        service.getPreviousVersions(noteId).unwrap()

    override suspend fun getNextVersions(noteId: String): List<NoteRecord> =
        service.getNextVersions(noteId).unwrap()

    override suspend fun getOtherVersions(noteId: String): List<NoteRecord> =
        service.getOtherVersions(noteId).unwrap()

    override suspend fun search(query: String, limit: UInt): List<NoteRecord> =
        service.search(query, limit).unwrap()

    override suspend fun searchTags(query: String, limit: UInt): List<String> =
        service.searchTags(query, limit).unwrap()

    override suspend fun getAllTags(): List<String> =
        service.getAllTags().unwrap()

    override suspend fun getNotesByTag(tag: String, cursor: String?, limit: UInt?): List<NoteRecord> =
        service.getNotesByTag(tag, cursor, limit).unwrap()

    override suspend fun getFilteredNotes(
        filter: NoteFeedFilter,
        cursor: String?,
        limit: UInt?,
    ): List<NoteRecord> = service.getFilteredNotes(filter, cursor, limit).unwrap()

    override suspend fun createNote(content: String, tags: List<String>): NoteRecord {
        val created = service.createNote(content, tags).unwrap()
        _mutations.tryEmit(SynapMutation.Created(created.id))
        return created
    }

    override suspend fun replyToNote(parentId: String, content: String, tags: List<String>): NoteRecord {
        val created = service.replyNote(parentId, content, tags).unwrap()
        _mutations.tryEmit(SynapMutation.Replied(parentId = parentId, noteId = created.id))
        return created
    }

    override suspend fun editNote(targetId: String, newContent: String, tags: List<String>): NoteRecord {
        val edited = service.editNote(targetId, newContent, tags).unwrap()
        _mutations.tryEmit(SynapMutation.Edited(oldId = targetId, newId = edited.id))
        return edited
    }

    override suspend fun deleteNote(targetId: String) {
        service.deleteNote(targetId).unwrap()
        _mutations.tryEmit(SynapMutation.Deleted(targetId))
    }

    override suspend fun restoreNote(targetId: String) {
        service.restoreNote(targetId).unwrap()
        _mutations.tryEmit(SynapMutation.Restored(targetId))
    }

    private fun <T> Result<T>.unwrap(): T = getOrElse { throw it }
}
