package com.synap.app.data.service

import android.content.Context
import com.synap.app.data.model.SyncConnectionRecord
import com.synap.app.data.model.SyncConnectionStatus
import dagger.hilt.android.qualifiers.ApplicationContext
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import org.json.JSONArray
import org.json.JSONObject

@Singleton
class SyncConnectionStore @Inject constructor(
    @ApplicationContext context: Context,
) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun list(): List<SyncConnectionRecord> {
        val raw = prefs.getString(KEY_CONNECTIONS, null) ?: return emptyList()
        val array = runCatching { JSONArray(raw) }.getOrNull() ?: return emptyList()
        return buildList {
            for (index in 0 until array.length()) {
                val item = array.optJSONObject(index) ?: continue
                add(item.toConnectionRecord())
            }
        }
    }

    fun create(name: String, host: String, port: Int): SyncConnectionRecord {
        val record = SyncConnectionRecord(
            id = UUID.randomUUID().toString(),
            name = name,
            host = host,
            port = port,
        )
        val updated = list().toMutableList().apply { add(record) }
        save(updated)
        return record
    }

    fun delete(connectionId: String) {
        val updated = list().filterNot { it.id == connectionId }
        save(updated)
    }

    fun update(record: SyncConnectionRecord) {
        val updated = list().map { existing ->
            if (existing.id == record.id) record else existing
        }
        save(updated)
    }

    private fun save(records: List<SyncConnectionRecord>) {
        val payload = JSONArray().apply {
            records.forEach { record -> put(record.toJson()) }
        }
        prefs.edit().putString(KEY_CONNECTIONS, payload.toString()).apply()
    }

    private fun JSONObject.toConnectionRecord(): SyncConnectionRecord = SyncConnectionRecord(
        id = optString("id"),
        name = optString("name"),
        host = optString("host"),
        port = optInt("port"),
        status = runCatching {
            SyncConnectionStatus.valueOf(optString("status"))
        }.getOrDefault(SyncConnectionStatus.Idle),
        statusMessage = optString("statusMessage", "已保存，尚未配对"),
    )

    private fun SyncConnectionRecord.toJson(): JSONObject = JSONObject().apply {
        put("id", id)
        put("name", name)
        put("host", host)
        put("port", port)
        put("status", status.name)
        put("statusMessage", statusMessage)
    }

    private companion object {
        const val PREFS_NAME = "sync_connections"
        const val KEY_CONNECTIONS = "connections"
    }
}
