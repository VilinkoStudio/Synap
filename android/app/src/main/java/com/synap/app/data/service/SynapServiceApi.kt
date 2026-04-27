package com.synap.app.data.service

import com.synap.app.data.model.NoteFeedFilter
import com.synap.app.data.model.LocalIdentity
import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.NoteVersionRecord
import com.synap.app.data.model.PeerRecord
import com.synap.app.data.model.PeerTrustStatus
import com.synap.app.data.model.SearchResultRecord
import com.synap.app.data.model.ShareImportStats
import com.synap.app.data.model.StarmapPointRecord
import com.synap.app.data.model.SyncSession
import com.synap.app.data.model.SyncSessionRecord
import com.synap.app.data.model.TimelineDirection
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

    suspend fun exportShare(noteIds: List<String>): Result<ByteArray>

    suspend fun importShare(bytes: ByteArray): Result<ShareImportStats>

    suspend fun getLocalIdentity(): Result<LocalIdentity>

    suspend fun getPeers(): Result<List<PeerRecord>>

    suspend fun trustPeer(publicKey: ByteArray, note: String?): Result<PeerRecord>

    suspend fun updatePeerNote(peerId: String, note: String?): Result<PeerRecord>

    suspend fun setPeerStatus(peerId: String, status: PeerTrustStatus): Result<PeerRecord>

    suspend fun deletePeer(peerId: String): Result<Unit>

    suspend fun getRecentSyncSessions(limit: UInt? = null): Result<List<SyncSessionRecord>>

    suspend fun initiateSync(transport: SyncTransportChannel): Result<SyncSession>

    suspend fun listenSync(transport: SyncTransportChannel): Result<SyncSession>

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

    suspend fun getPreviousVersions(noteId: String): Result<List<NoteVersionRecord>>

    suspend fun getNextVersions(noteId: String): Result<List<NoteVersionRecord>>

    suspend fun getOtherVersions(noteId: String): Result<List<NoteVersionRecord>>

    suspend fun getDeletedNotes(cursor: String?, limit: UInt?): Result<List<NoteRecord>>

    suspend fun getStarmap(): Result<List<StarmapPointRecord>>

    suspend fun search(query: String, limit: UInt): Result<List<NoteRecord>>

    suspend fun searchFusion(
        query: String,
        limit: UInt,
        fuzzyLimit: UInt? = null,
        semanticLimit: UInt? = null,
    ): Result<List<SearchResultRecord>>

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
