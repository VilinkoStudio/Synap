package com.fuwaki.synap

import org.json.JSONArray
import org.json.JSONObject

/**
 * Pure JNI bridge for Synap Core FFI
 * Uses standard JNI without JNA dependency
 * Complex types are returned as JSON byte arrays
 */
object SynapCore {
    init {
        System.loadLibrary("uniffi_synap_coreffi")
    }

    // ==================== Lifecycle ====================

    /** Open a file-based database */
    external fun open(path: String): Long

    /** Open an in-memory database */
    external fun openMemory(): Long

    /** Close the database and release resources */
    external fun close(handle: Long)

    /** Free memory for JSON responses (kept for compatibility, no-op) */
    external fun freeBytes(address: Long, size: Long)

    // ==================== Thought Operations ====================

    /** Add a new thought. Returns JSON representation of NoteView */
    external fun addThought(handle: Long, content: String): ByteArray?

    /** Get a thought by ID. Returns JSON representation of NoteView */
    external fun getThought(handle: Long, id: ByteArray): ByteArray?

    /** Edit an existing thought. Returns JSON representation of NoteView */
    external fun editThought(handle: Long, id: ByteArray, newContent: String): ByteArray?

    /** Abandon (soft delete) a thought */
    external fun abandonThought(handle: Long, id: ByteArray): Boolean

    /** List all non-abandoned thoughts. Returns JSON array of NoteView */
    external fun listThoughts(handle: Long): ByteArray?

    /** List thoughts with pagination. Returns JSON array of NoteView */
    external fun listThoughtsPaginated(handle: Long, offset: Int, limit: Int): ByteArray?

    // ==================== Search ====================

    /** Search thoughts by content. Returns JSON array of NoteView */
    external fun searchThoughts(handle: Long, query: String): ByteArray?

    // ==================== Reply/DAG Operations ====================

    /** Reply to a thought. Returns JSON representation of NoteView */
    external fun replyThought(handle: Long, parentId: ByteArray, content: String): ByteArray?

    /** Get all replies to a thought. Returns JSON array of NoteView */
    external fun getReplies(handle: Long, parentId: ByteArray): ByteArray?

    /** Get parents of a thought. Returns JSON array of NoteView */
    external fun getParents(handle: Long, childId: ByteArray): ByteArray?

    /** Get knowledge graph from a root. Returns JSON array of GraphEntry */
    external fun getGraph(handle: Long, rootId: ByteArray, maxDepth: Int): ByteArray?

    // ==================== Tag Operations ====================

    /** Add a tag to a thought */
    external fun addTag(handle: Long, thoughtId: ByteArray, tag: String): Boolean

    /** Remove a tag from a thought */
    external fun removeTag(handle: Long, thoughtId: ByteArray, tag: String): Boolean

    /** Get all unique tags. Returns JSON array of strings */
    external fun getAllTags(handle: Long): ByteArray?

    /** Get thoughts by tag. Returns JSON array of NoteView */
    external fun getThoughtsByTag(handle: Long, tag: String): ByteArray?

    /** Get tags for a thought. Returns JSON array of strings */
    external fun getTagsForThought(handle: Long, thoughtId: ByteArray): ByteArray?

    // ==================== Analytics & Maintenance ====================

    /** Get service statistics. Returns JSON representation of ServiceStats */
    external fun getStats(handle: Long): ByteArray?

    /** Find notes by ULID prefix. Returns JSON array of NoteView */
    external fun findNotesByPrefix(handle: Long, prefix: String): ByteArray?

    /** Get note by short ID. Returns JSON representation of NoteView */
    external fun getNoteByShortId(handle: Long, shortId: String): ByteArray?

    /** Execute garbage collection. Returns JSON representation of FfiGcResult */
    external fun scrubGarbage(handle: Long): ByteArray?

    // ==================== Helper Methods ====================

    /** Parse JSON byte array to JSONObject or JSONArray */
    fun parseJson(jsonBytes: ByteArray?): Any? {
        if (jsonBytes == null || jsonBytes.isEmpty()) return null
        val jsonStr = String(jsonBytes, Charsets.UTF_8)
        return try {
            when {
                jsonStr.trimStart().startsWith("[") -> JSONArray(jsonStr)
                jsonStr.trimStart().startsWith("{") -> JSONObject(jsonStr)
                else -> jsonStr
            }
        } catch (e: Exception) {
            null
        }
    }

    /** Parse JSON byte array to NoteView */
    fun parseNoteView(jsonBytes: ByteArray?): NoteViewData? {
        val json = parseJson(jsonBytes) as? JSONObject ?: return null
        return NoteViewData(
            id = json.getJSONArray("id").toByteArray(),
            content = json.getString("content"),
            updatedAt = json.getLong("updated_at"),
            createdAt = json.getLong("created_at"),
            deleted = json.getBoolean("deleted"),
            tags = json.getJSONArray("tags").toStringList()
        )
    }

    /** Parse JSON byte array to list of NoteView */
    fun parseNoteViewList(jsonBytes: ByteArray?): List<NoteViewData> {
        val json = parseJson(jsonBytes) as? JSONArray ?: return emptyList()
        return (0 until json.length()).mapNotNull { i ->
            parseNoteView(json.getJSONArray(i).toString().toByteArray(Charsets.UTF_8))
        }
    }

    /** Convert JSONArray to List<String> */
    private fun JSONArray.toStringList(): List<String> {
        return (0 until length()).map { getString(it) }
    }

    /** Convert JSONArray to ByteArray */
    private fun JSONArray.toByteArray(): ByteArray {
        return toString().toByteArray(Charsets.UTF_8)
    }
}

/**
 * Data class representing NoteView from Rust
 */
data class NoteViewData(
    val id: ByteArray,
    val content: String,
    val updatedAt: Long,
    val createdAt: Long,
    val deleted: Boolean,
    val tags: List<String>
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as NoteViewData

        if (!id.contentEquals(other.id)) return false
        if (content != other.content) return false
        if (updatedAt != other.updatedAt) return false
        if (createdAt != other.createdAt) return false
        if (deleted != other.deleted) return false
        if (tags != other.tags) return false

        return true
    }

    override fun hashCode(): Int {
        var result = id.contentHashCode()
        result = 31 * result + content.hashCode()
        result = 31 * result + updatedAt.hashCode()
        result = 31 * result + createdAt.hashCode()
        result = 31 * result + deleted.hashCode()
        result = 31 * result + tags.hashCode()
        return result
    }
}
