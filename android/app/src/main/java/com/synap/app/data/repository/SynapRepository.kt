package com.synap.app.data.repository

import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.NoteNeighborsRecord
import com.synap.app.data.model.NoteDraftRecord
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.NoteSegmentDirection
import com.synap.app.data.model.NoteSegmentRecord
import com.synap.app.data.model.NoteVersionRecord
import com.synap.app.data.model.ReplyItem
import com.synap.app.data.model.SearchResultRecord
import com.synap.app.data.model.ShareImportStats
import com.synap.app.data.model.TimelineDirection
import com.synap.app.data.model.TimelineDensityPointRecord
import com.synap.app.data.portal.CursorPortal
import com.synap.app.data.service.SynapServiceApi
import com.synap.app.data.service.LegacyDraftStore
import com.synap.app.data.service.LegacyDraftOrigin
import com.synap.app.data.service.LegacyDraftImportTarget
import com.synap.app.data.service.colorCss
import com.synap.app.data.service.migrateLegacyDraftRecords
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow

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
    val draftChanges: SharedFlow<Unit>

    suspend fun initialize()

    suspend fun shutdown()

    fun openRecentPortal(limit: UInt = 20u): CursorPortal<NoteRecord>

    fun openRepliesPortal(parentId: String, limit: UInt = 20u): CursorPortal<ReplyItem>

    fun openDeletedPortal(limit: UInt = 20u): CursorPortal<NoteRecord>

    fun openTaggedPortal(tag: String, limit: UInt = 20u): CursorPortal<NoteRecord>

    fun openFilteredPortal(
        filter: NoteFeedFilter,
        limit: UInt = 20u,
    ): CursorPortal<NoteRecord>

    fun openTimelinePortal(
        filter: NoteFeedFilter,
        limit: UInt = 20u,
    ): CursorPortal<NoteRecord>

    fun openTimelineAroundPortal(
        filter: NoteFeedFilter,
        timestampMs: ULong,
        direction: TimelineDirection = TimelineDirection.Older,
        limit: UInt = 20u,
    ): CursorPortal<NoteRecord>

    suspend fun getTimelineDensity(
        filter: NoteFeedFilter,
        startMs: ULong,
        endMs: ULong,
        bucketMs: ULong,
    ): List<TimelineDensityPointRecord>

    suspend fun getNote(idOrShortId: String): NoteRecord

    suspend fun getOrigins(noteId: String): List<NoteRecord>

    suspend fun getNoteSegment(
        anchorId: String,
        direction: NoteSegmentDirection,
    ): NoteSegmentRecord

    suspend fun getNoteNeighbors(noteId: String): NoteNeighborsRecord

    suspend fun getPreviousVersions(noteId: String): List<NoteVersionRecord>

    suspend fun getNextVersions(noteId: String): List<NoteVersionRecord>

    suspend fun getOtherVersions(noteId: String): List<NoteVersionRecord>

    suspend fun search(query: String, limit: UInt = 50u): List<NoteRecord>

    suspend fun backfillNoteEmbeddings(): ULong

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

    suspend fun setNoteColor(targetId: String, color: String?): NoteRecord

    suspend fun createDraft(): NoteDraftRecord

    suspend fun createEditDraft(noteId: String): NoteDraftRecord

    suspend fun createReplyDraft(parentId: String): NoteDraftRecord

    suspend fun getDraft(draftId: String): NoteDraftRecord

    suspend fun listDrafts(): List<NoteDraftRecord>

    suspend fun persistDraft(draftId: String): NoteDraftRecord

    suspend fun discardDraft(draftId: String)

    suspend fun updateDraft(
        draftId: String,
        content: String? = null,
        tags: List<String>? = null,
        color: String? = null,
        updateColor: Boolean = false,
        expectedRevision: ULong? = null,
    ): NoteDraftRecord

    suspend fun commitDraft(draftId: String): NoteRecord

    suspend fun deleteNote(targetId: String)

    suspend fun restoreNote(targetId: String)
}

@Singleton
class SynapRepositoryImpl @Inject constructor(
    private val service: SynapServiceApi,
    private val mutationStore: SynapMutationStore,
    private val legacyDraftStore: LegacyDraftStore,
) : SynapRepository {
    override val mutations: SharedFlow<SynapMutation> = mutationStore.mutations
    private val mutableDraftChanges = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
    override val draftChanges: SharedFlow<Unit> = mutableDraftChanges.asSharedFlow()

    override suspend fun initialize() {
        service.initialize().unwrap()
        migrateLegacyDrafts()
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

    override fun openTimelinePortal(filter: NoteFeedFilter, limit: UInt): CursorPortal<NoteRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                service.getTimelineNotesPage(
                    filter = filter,
                    cursor = cursor,
                    direction = TimelineDirection.Older,
                    limit = pageLimit.takeIf { it > 0u },
                ).unwrap()
            },
        )

    override fun openTimelineAroundPortal(
        filter: NoteFeedFilter,
        timestampMs: ULong,
        direction: TimelineDirection,
        limit: UInt,
    ): CursorPortal<NoteRecord> =
        CursorPortal(
            limit = limit,
            fetchPage = { cursor, pageLimit ->
                if (cursor == null) {
                    service.getTimelineNotesAround(
                        filter = filter,
                        timestampMs = timestampMs,
                        direction = direction,
                        limit = pageLimit.takeIf { it > 0u },
                    ).unwrap()
                } else {
                    service.getTimelineNotesPage(
                        filter = filter,
                        cursor = cursor,
                        direction = direction,
                        limit = pageLimit.takeIf { it > 0u },
                    ).unwrap()
                }
            },
        )

    override suspend fun getTimelineDensity(
        filter: NoteFeedFilter,
        startMs: ULong,
        endMs: ULong,
        bucketMs: ULong,
    ): List<TimelineDensityPointRecord> =
        service.getTimelineDensity(filter, startMs, endMs, bucketMs).unwrap()

    override suspend fun getNote(idOrShortId: String): NoteRecord =
        service.getNote(idOrShortId).unwrap()

    override suspend fun getOrigins(noteId: String): List<NoteRecord> =
        service.getOrigins(noteId).unwrap()

    override suspend fun getNoteSegment(
        anchorId: String,
        direction: NoteSegmentDirection,
    ): NoteSegmentRecord = service.getNoteSegment(anchorId, direction).unwrap()

    override suspend fun getNoteNeighbors(noteId: String): NoteNeighborsRecord =
        service.getNoteNeighbors(noteId).unwrap()

    override suspend fun getPreviousVersions(noteId: String): List<NoteVersionRecord> =
        service.getPreviousVersions(noteId).unwrap()

    override suspend fun getNextVersions(noteId: String): List<NoteVersionRecord> =
        service.getNextVersions(noteId).unwrap()

    override suspend fun getOtherVersions(noteId: String): List<NoteVersionRecord> =
        service.getOtherVersions(noteId).unwrap()

    override suspend fun search(query: String, limit: UInt): List<NoteRecord> =
        service.search(query, limit).unwrap()

    override suspend fun backfillNoteEmbeddings(): ULong =
        service.backfillNoteEmbeddings().unwrap()

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

    override suspend fun setNoteColor(targetId: String, color: String?): NoteRecord {
        val edited = service.setNoteColor(targetId, color).unwrap()
        mutationStore.emit(SynapMutation.Edited(oldId = targetId, newId = edited.id))
        return edited
    }

    override suspend fun createDraft(): NoteDraftRecord = service.draftNew().unwrap()

    override suspend fun createEditDraft(noteId: String): NoteDraftRecord =
        service.draftFromNote(noteId).unwrap()

    override suspend fun createReplyDraft(parentId: String): NoteDraftRecord =
        service.draftReplyTo(parentId).unwrap()

    override suspend fun getDraft(draftId: String): NoteDraftRecord =
        service.draftGet(draftId).unwrap()

    override suspend fun listDrafts(): List<NoteDraftRecord> = service.draftList().unwrap()

    override suspend fun persistDraft(draftId: String): NoteDraftRecord {
        val persisted = service.draftPersist(draftId).unwrap()
        mutableDraftChanges.emit(Unit)
        return persisted
    }

    override suspend fun discardDraft(draftId: String) {
        service.draftDiscard(draftId).unwrap()
        mutableDraftChanges.emit(Unit)
    }

    override suspend fun updateDraft(
        draftId: String,
        content: String?,
        tags: List<String>?,
        color: String?,
        updateColor: Boolean,
        expectedRevision: ULong?,
    ): NoteDraftRecord {
        val updated = service
            .draftUpdate(draftId, content, tags, color, updateColor, expectedRevision)
            .unwrap()
        if (updated.persisted) {
            mutableDraftChanges.emit(Unit)
        }
        return updated
    }

    override suspend fun commitDraft(draftId: String): NoteRecord {
        val draft = getDraft(draftId)
        val note = service.draftCommit(draftId).unwrap()
        mutableDraftChanges.emit(Unit)
        when {
            draft.editedFrom != null -> mutationStore.emit(
                SynapMutation.Edited(oldId = draft.editedFrom, newId = note.id),
            )
            draft.replyTo != null -> mutationStore.emit(
                SynapMutation.Replied(parentId = draft.replyTo, noteId = note.id),
            )
            else -> mutationStore.emit(SynapMutation.Created(note.id))
        }
        return note
    }

    override suspend fun deleteNote(targetId: String) {
        service.deleteNote(targetId).unwrap()
        mutationStore.emit(SynapMutation.Deleted(targetId))
    }

    override suspend fun restoreNote(targetId: String) {
        service.restoreNote(targetId).unwrap()
        mutationStore.emit(SynapMutation.Restored(targetId))
    }

    private suspend fun migrateLegacyDrafts() {
        migrateLegacyDraftRecords(
            // Core timestamps each imported update; oldest-first preserves legacy ordering.
            records = legacyDraftStore.listAll().asReversed(),
            target = object : LegacyDraftImportTarget {
                override suspend fun create(origin: LegacyDraftOrigin): String =
                    when (origin) {
                        LegacyDraftOrigin.Create -> createDraft()
                        is LegacyDraftOrigin.Reply -> createReplyDraft(origin.parentId)
                        is LegacyDraftOrigin.Edit -> createEditDraft(origin.noteId)
                    }.id

                override suspend fun updateAndPersist(
                    draftId: String,
                    legacy: com.synap.app.data.service.DraftRecord,
                ) {
                    val updated = updateDraft(
                    draftId = draftId,
                    content = legacy.content,
                    tags = legacy.tags,
                    color = legacy.colorCss(),
                    updateColor = true,
                )
                    persistDraft(updated.id)
                }
            },
            onMigrated = legacyDraftStore::delete,
        )
    }

    private fun <T> Result<T>.unwrap(): T = getOrElse { throw it }
}
