package com.synap.app.data.service

internal interface LegacyDraftImportTarget {
    suspend fun create(origin: LegacyDraftOrigin): String

    suspend fun updateAndPersist(draftId: String, legacy: DraftRecord)
}

internal suspend fun migrateLegacyDraftRecords(
    records: List<DraftRecord>,
    target: LegacyDraftImportTarget,
    onMigrated: (String) -> Unit,
) {
    records.forEach { legacy ->
        runCatching {
            val origin = legacy.origin()
            val draftId = runCatching { target.create(origin) }.getOrElse { error ->
                if (origin is LegacyDraftOrigin.Create) throw error
                target.create(LegacyDraftOrigin.Create)
            }
            target.updateAndPersist(draftId, legacy)
        }.onSuccess {
            onMigrated(legacy.id)
        }
    }
}
