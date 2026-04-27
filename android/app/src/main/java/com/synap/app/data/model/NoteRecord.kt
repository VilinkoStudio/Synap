package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteBriefDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteTagDiffDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteContentDiffStatsDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteTextChangeDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteTextChangeKindDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteVersionDiffDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteVersionDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.TimelineNotesPageDto
import com.synap.app.data.portal.CursorPage

data class NoteBriefRecord(
    val id: String,
    val contentPreview: String,
    val createdAt: Long,
) {
    companion object {
        fun fromDto(dto: NoteBriefDto): NoteBriefRecord = NoteBriefRecord(
            id = dto.id,
            contentPreview = dto.contentPreview,
            createdAt = dto.createdAt,
        )
    }
}

data class NoteRecord(
    val id: String,
    val content: String,
    val tags: List<String>,
    val createdAt: Long,
    val deleted: Boolean,
    val replyTo: NoteBriefRecord? = null,
    val editedFrom: NoteBriefRecord? = null,
) {
    companion object {
        fun fromDto(dto: NoteDto): NoteRecord = NoteRecord(
            id = dto.id,
            content = dto.content,
            tags = dto.tags,
            createdAt = dto.createdAt,
            deleted = dto.deleted,
            replyTo = dto.replyTo?.let(NoteBriefRecord::fromDto),
            editedFrom = dto.editedFrom?.let(NoteBriefRecord::fromDto),
        )
    }
}

enum class NoteTextChangeKind {
    Equal,
    Insert,
    Delete,
    ;

    companion object {
        fun fromDto(dto: NoteTextChangeKindDto): NoteTextChangeKind = when (dto) {
            NoteTextChangeKindDto.EQUAL -> Equal
            NoteTextChangeKindDto.INSERT -> Insert
            NoteTextChangeKindDto.DELETE -> Delete
        }
    }
}

data class NoteTextChangeRecord(
    val kind: NoteTextChangeKind,
    val value: String,
) {
    companion object {
        fun fromDto(dto: NoteTextChangeDto): NoteTextChangeRecord = NoteTextChangeRecord(
            kind = NoteTextChangeKind.fromDto(dto.kind),
            value = dto.value,
        )
    }
}

data class NoteTagDiffRecord(
    val added: List<String>,
    val removed: List<String>,
) {
    companion object {
        fun fromDto(dto: NoteTagDiffDto): NoteTagDiffRecord = NoteTagDiffRecord(
            added = dto.added,
            removed = dto.removed,
        )
    }
}

data class NoteContentDiffStatsRecord(
    val insertedChars: UInt,
    val deletedChars: UInt,
    val insertedLines: UInt,
    val deletedLines: UInt,
) {
    companion object {
        fun fromDto(dto: NoteContentDiffStatsDto): NoteContentDiffStatsRecord =
            NoteContentDiffStatsRecord(
                insertedChars = dto.insertedChars,
                deletedChars = dto.deletedChars,
                insertedLines = dto.insertedLines,
                deletedLines = dto.deletedLines,
            )
    }
}

data class NoteVersionDiffRecord(
    val tags: NoteTagDiffRecord,
    val content: List<NoteTextChangeRecord>,
    val contentSummary: List<NoteTextChangeRecord>,
    val contentStats: NoteContentDiffStatsRecord,
) {
    companion object {
        fun fromDto(dto: NoteVersionDiffDto): NoteVersionDiffRecord = NoteVersionDiffRecord(
            tags = NoteTagDiffRecord.fromDto(dto.tags),
            content = dto.content.map(NoteTextChangeRecord::fromDto),
            contentSummary = dto.contentSummary.map(NoteTextChangeRecord::fromDto),
            contentStats = NoteContentDiffStatsRecord.fromDto(dto.contentStats),
        )
    }
}

data class NoteVersionRecord(
    val note: NoteRecord,
    val diff: NoteVersionDiffRecord,
) {
    companion object {
        fun fromDto(dto: NoteVersionDto): NoteVersionRecord = NoteVersionRecord(
            note = NoteRecord.fromDto(dto.note),
            diff = NoteVersionDiffRecord.fromDto(dto.diff),
        )
    }
}

data class ReplyItem(
    val note: NoteRecord,
    val parentId: String,
)

internal fun NoteDto.toNoteRecord(): NoteRecord = NoteRecord.fromDto(this)

internal fun List<NoteDto>.toNoteRecords(): List<NoteRecord> = map(NoteRecord::fromDto)

internal fun NoteVersionDto.toNoteVersionRecord(): NoteVersionRecord = NoteVersionRecord.fromDto(this)

internal fun List<NoteVersionDto>.toNoteVersionRecords(): List<NoteVersionRecord> =
    map(NoteVersionRecord::fromDto)

internal fun TimelineNotesPageDto.toCursorPage(): CursorPage<NoteRecord> = CursorPage(
    items = notes.toNoteRecords(),
    nextCursor = nextCursor,
)
