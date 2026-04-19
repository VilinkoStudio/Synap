package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.ShareStatsDto

data class ShareImportStats(
    val records: Long,
    val recordsApplied: Long,
    val bytes: Long,
    val durationMs: Long,
) {
    companion object {
        fun fromDto(dto: ShareStatsDto): ShareImportStats = ShareImportStats(
            records = dto.records.toLong(),
            recordsApplied = dto.recordsApplied.toLong(),
            bytes = dto.bytes.toLong(),
            durationMs = dto.durationMs.toLong(),
        )
    }
}

internal fun ShareStatsDto.toShareImportStats(): ShareImportStats = ShareImportStats.fromDto(this)
