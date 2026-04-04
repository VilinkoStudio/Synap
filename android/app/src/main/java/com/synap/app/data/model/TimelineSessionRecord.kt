package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.TimelineSessionDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.TimelineSessionsPageDto
import com.synap.app.data.portal.CursorPage

data class TimelineSessionRecord(
    val startedAt: Long,
    val endedAt: Long,
    val noteCount: Int,
    val notes: List<NoteRecord>,
) {
    companion object {
        fun fromDto(dto: TimelineSessionDto): TimelineSessionRecord = TimelineSessionRecord(
            startedAt = dto.startedAt,
            endedAt = dto.endedAt,
            noteCount = dto.noteCount.toInt(),
            notes = dto.notes.toNoteRecords(),
        )
    }
}

internal fun TimelineSessionDto.toTimelineSessionRecord(): TimelineSessionRecord =
    TimelineSessionRecord.fromDto(this)

internal fun List<TimelineSessionDto>.toTimelineSessionRecords(): List<TimelineSessionRecord> =
    map(TimelineSessionRecord::fromDto)

internal fun TimelineSessionsPageDto.toCursorPage(): CursorPage<TimelineSessionRecord> = CursorPage(
    items = sessions.toTimelineSessionRecords(),
    nextCursor = nextCursor,
)
