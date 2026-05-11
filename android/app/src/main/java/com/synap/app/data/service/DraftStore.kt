package com.synap.app.data.service

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import org.json.JSONArray
import org.json.JSONObject

data class DraftRecord(
    val id: String = UUID.randomUUID().toString(),
    val content: String,
    val tags: List<String>,
    val noteColorHue: Float? = null,
    val mode: String = "create", // "create", "reply", "edit"
    val parentId: String? = null,
    val parentSummary: String? = null,
    val editNoteId: String? = null,
    val savedAt: Long = System.currentTimeMillis(),
    val reason: String = "auto", // "auto" (auto-save) or "manual" (user exited without saving)
    val status: String = "pending", // "editing", "pending", "read"
)

@Singleton
class DraftStore @Inject constructor(
    @ApplicationContext context: Context,
) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    private val settingsPrefs = context.getSharedPreferences("synap_settings", Context.MODE_PRIVATE)

    fun getCapacity(): Int {
        return settingsPrefs.getInt(KEY_CAPACITY, DEFAULT_CAPACITY)
    }

    fun setCapacity(capacity: Int) {
        settingsPrefs.edit().putInt(KEY_CAPACITY, capacity).apply()
    }

    fun isEnabled(): Boolean {
        return getCapacity() > 0
    }

    fun list(): List<DraftRecord> {
        if (!isEnabled()) return emptyList()
        val raw = prefs.getString(KEY_DRAFTS, null) ?: return emptyList()
        val array = runCatching { JSONArray(raw) }.getOrNull() ?: return emptyList()
        return buildList {
            for (index in 0 until array.length()) {
                val item = array.optJSONObject(index) ?: continue
                add(item.toDraftRecord())
            }
        }
        .filter { it.status != "editing" } // 过滤掉编辑中的草稿
        .sortedByDescending { it.savedAt }
    }

    fun listAll(): List<DraftRecord> {
        val raw = prefs.getString(KEY_DRAFTS, null) ?: return emptyList()
        val array = runCatching { JSONArray(raw) }.getOrNull() ?: return emptyList()
        return buildList {
            for (index in 0 until array.length()) {
                val item = array.optJSONObject(index) ?: continue
                add(item.toDraftRecord())
            }
        }.sortedByDescending { it.savedAt }
    }

    fun save(draft: DraftRecord): DraftRecord {
        if (!isEnabled()) return draft
        val current = listAll().toMutableList()
        val capacity = getCapacity()

        // Check if draft with same ID exists, update it
        val existingIndex = current.indexOfFirst { it.id == draft.id }
        if (existingIndex != -1) {
            current[existingIndex] = draft
        } else {
            // Add new draft
            current.add(draft)
        }

        // Sort by time descending and trim to capacity
        val trimmed = current.sortedByDescending { it.savedAt }.take(capacity)

        saveList(trimmed)
        return draft
    }

    fun delete(id: String) {
        val current = listAll().toMutableList()
        current.removeAll { it.id == id }
        saveList(current)
    }

    fun updateStatus(id: String, status: String) {
        val current = listAll().toMutableList()
        val index = current.indexOfFirst { it.id == id }
        if (index != -1) {
            current[index] = current[index].copy(status = status)
            saveList(current)
        }
    }

    fun getLatestDraft(): DraftRecord? {
        return listAll().firstOrNull()
    }

    fun clear() {
        prefs.edit().remove(KEY_DRAFTS).apply()
    }

    fun count(): Int {
        return list().size
    }

    private fun saveList(records: List<DraftRecord>) {
        val payload = JSONArray().apply {
            records.forEach { record -> put(record.toJson()) }
        }
        prefs.edit().putString(KEY_DRAFTS, payload.toString()).apply()
    }

    private fun JSONObject.toDraftRecord(): DraftRecord = DraftRecord(
        id = optString("id", UUID.randomUUID().toString()),
        content = optString("content", ""),
        tags = optJSONArray("tags")?.let { arr ->
            (0 until arr.length()).mapNotNull { arr.optString(it) }
        } ?: emptyList(),
        noteColorHue = if (has("noteColorHue")) optDouble("noteColorHue").toFloat() else null,
        mode = optString("mode", "create"),
        parentId = optString("parentId", null),
        parentSummary = optString("parentSummary", null),
        editNoteId = optString("editNoteId", null),
        savedAt = optLong("savedAt", System.currentTimeMillis()),
        reason = optString("reason", "auto"),
        status = optString("status", "pending"),
    )

    private fun DraftRecord.toJson(): JSONObject = JSONObject().apply {
        put("id", id)
        put("content", content)
        put("tags", JSONArray().apply { tags.forEach { put(it) } })
        noteColorHue?.let { put("noteColorHue", it.toDouble()) }
        put("mode", mode)
        parentId?.let { put("parentId", it) }
        parentSummary?.let { put("parentSummary", it) }
        editNoteId?.let { put("editNoteId", it) }
        put("savedAt", savedAt)
        put("reason", reason)
        put("status", status)
    }

    companion object {
        private const val PREFS_NAME = "drafts"
        private const val KEY_DRAFTS = "draft_list"
        private const val KEY_CAPACITY = "draftCapacity"
        private const val DEFAULT_CAPACITY = 20
    }
}
