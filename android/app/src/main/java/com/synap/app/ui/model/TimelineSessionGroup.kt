package com.synap.app.ui.model

import com.synap.app.data.model.TimelineSessionRecord

data class TimelineSessionGroup(
    val startedAt: Long,
    val endedAt: Long,
    val noteCount: Int,
    val notes: List<Note>,
)

fun TimelineSessionRecord.toUiSessionGroup(): TimelineSessionGroup = TimelineSessionGroup(
    startedAt = startedAt,
    endedAt = endedAt,
    noteCount = noteCount,
    notes = notes.map { it.toUiNote() },
)
