package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteDto

data class NoteRecord(
    val id: String,
    val content: String,
    val tags: List<String>,
    val createdAt: Long,
) {
    companion object {
        fun fromDto(dto: NoteDto): NoteRecord = NoteRecord(
            id = dto.id,
            content = dto.content,
            tags = dto.tags,
            createdAt = dto.createdAt,
        )
    }
}

data class ReplyItem(
    val note: NoteRecord,
    val parentId: String,
)

internal fun NoteDto.toNoteRecord(): NoteRecord = NoteRecord.fromDto(this)

internal fun List<NoteDto>.toNoteRecords(): List<NoteRecord> = map(NoteRecord::fromDto)
