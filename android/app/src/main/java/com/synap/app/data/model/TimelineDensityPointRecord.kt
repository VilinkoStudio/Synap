package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.TimelineDensityPointDto

data class TimelineDensityPointRecord(
    val startedAt: Long,
    val endedAt: Long,
    val noteCount: Int,
) {
    companion object {
        fun fromDto(dto: TimelineDensityPointDto): TimelineDensityPointRecord =
            TimelineDensityPointRecord(
                startedAt = dto.startedAt,
                endedAt = dto.endedAt,
                noteCount = dto.noteCount.toInt(),
            )
    }
}

internal fun List<TimelineDensityPointDto>.toTimelineDensityPointRecords(): List<TimelineDensityPointRecord> =
    map(TimelineDensityPointRecord::fromDto)
