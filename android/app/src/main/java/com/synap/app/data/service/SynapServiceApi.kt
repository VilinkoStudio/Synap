package com.synap.app.data.service

import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.TimelineDirection
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.TimelineSessionRecord
import com.synap.app.data.portal.CursorPage
import java.io.InputStream
import java.io.OutputStream

interface SynapServiceApi {
    val isInitialized: Boolean

    suspend fun initialize(): Result<Unit>

    suspend fun initializeInMemory(): Result<Unit>

    suspend fun close(): Result<Unit>

    suspend fun exportDatabase(outputStream: OutputStream): Result<Unit>

    suspend fun replaceDatabase(inputStream: InputStream): Result<Unit>

    suspend fun getNote(idOrShortId: String): Result<NoteRecord>

    suspend fun getReplies(parentId: String, cursor: String?, limit: UInt): Result<List<NoteRecord>>

    suspend fun getRecentNote(cursor: String?, limit: UInt?): Result<List<NoteRecord>>

    suspend fun getRecentNotesPage(
        cursor: String?,
        direction: TimelineDirection,
        limit: UInt?,
    ): Result<CursorPage<NoteRecord>>

    suspend fun getRecentSessionsPage(
        cursor: String?,
        limit: UInt?,
    ): Result<CursorPage<TimelineSessionRecord>>

    suspend fun getOrigins(childId: String): Result<List<NoteRecord>>

    suspend fun getPreviousVersions(noteId: String): Result<List<NoteRecord>>

    suspend fun getNextVersions(noteId: String): Result<List<NoteRecord>>

    suspend fun getOtherVersions(noteId: String): Result<List<NoteRecord>>

    suspend fun getDeletedNotes(cursor: String?, limit: UInt?): Result<List<NoteRecord>>

    suspend fun search(query: String, limit: UInt): Result<List<NoteRecord>>

    suspend fun searchTags(query: String, limit: UInt): Result<List<String>>

    suspend fun recommendTag(content: String, limit: UInt): Result<List<String>>

    suspend fun getAllTags(): Result<List<String>>

    suspend fun getNotesByTag(tag: String, cursor: String?, limit: UInt?): Result<List<NoteRecord>>

    suspend fun getFilteredNotes(
        filter: NoteFeedFilter,
        cursor: String?,
        limit: UInt?,
    ): Result<List<NoteRecord>>

    suspend fun getFilteredNotesPage(
        filter: NoteFeedFilter,
        cursor: String?,
        direction: TimelineDirection,
        limit: UInt?,
    ): Result<CursorPage<NoteRecord>>

    suspend fun createNote(content: String, tags: List<String>): Result<NoteRecord>

    suspend fun replyNote(parentId: String, content: String, tags: List<String>): Result<NoteRecord>

    suspend fun editNote(targetId: String, newContent: String, tags: List<String>): Result<NoteRecord>

    suspend fun deleteNote(targetId: String): Result<Unit>

    suspend fun restoreNote(targetId: String): Result<Unit>
}
