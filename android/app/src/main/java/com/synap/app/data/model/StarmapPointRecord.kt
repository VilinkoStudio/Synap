package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.StarmapPointDto

data class StarmapPointRecord(
    val id: String,
    val x: Float,
    val y: Float,
) {
    companion object {
        fun fromDto(dto: StarmapPointDto): StarmapPointRecord = StarmapPointRecord(
            id = dto.id,
            x = dto.x,
            y = dto.y,
        )
    }
}

internal fun StarmapPointDto.toStarmapPointRecord(): StarmapPointRecord =
    StarmapPointRecord.fromDto(this)

internal fun List<StarmapPointDto>.toStarmapPoints(): List<StarmapPointRecord> =
    map(StarmapPointRecord::fromDto)
