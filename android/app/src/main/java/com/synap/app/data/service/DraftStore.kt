package com.synap.app.data.service

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.roundToInt

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

internal sealed interface LegacyDraftOrigin {
    data object Create : LegacyDraftOrigin
    data class Reply(val parentId: String) : LegacyDraftOrigin
    data class Edit(val noteId: String) : LegacyDraftOrigin
}

internal fun DraftRecord.origin(): LegacyDraftOrigin = when {
    mode == "edit" && !editNoteId.isNullOrBlank() -> LegacyDraftOrigin.Edit(editNoteId)
    mode == "reply" && !parentId.isNullOrBlank() -> LegacyDraftOrigin.Reply(parentId)
    else -> LegacyDraftOrigin.Create
}

internal fun DraftRecord.colorCss(): String? {
    val hue = noteColorHue ?: return null
    val normalized = ((hue % 360f) + 360f) % 360f
    val sector = normalized / 60f
    val x = (1f - kotlin.math.abs(sector % 2f - 1f)) * 255f
    val (red, green, blue) = when (sector.toInt()) {
        0 -> Triple(255f, x, 0f)
        1 -> Triple(x, 255f, 0f)
        2 -> Triple(0f, 255f, x)
        3 -> Triple(0f, x, 255f)
        4 -> Triple(x, 0f, 255f)
        else -> Triple(255f, 0f, x)
    }
    return "#%02x%02x%02x".format(red.roundToInt(), green.roundToInt(), blue.roundToInt())
}

@Singleton
class LegacyDraftStore @Inject constructor(
    @ApplicationContext context: Context,
) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

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

    fun delete(id: String) {
        val current = listAll().toMutableList()
        current.removeAll { it.id == id }
        saveList(current)
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
        parentId = optString("parentId").takeIf(String::isNotBlank),
        parentSummary = optString("parentSummary").takeIf(String::isNotBlank),
        editNoteId = optString("editNoteId").takeIf(String::isNotBlank),
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
    }
}
