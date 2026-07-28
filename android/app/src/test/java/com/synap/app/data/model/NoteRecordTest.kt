package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteBriefDto
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class NoteRecordTest {
    @Test
    fun mapsFromGeneratedDto() {
        val replyTo = NoteBriefDto(
            id = "01ARZ3NDEKTSV4RRFFQ69G5FAA",
            contentPreview = "parent preview",
            createdAt = 1200L,
        )
        val dto = NoteDto(
            id = "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            content = "hello",
            tags = listOf("rust", "android"),
            color = "#123456",
            createdAt = 1234L,
            deleted = true,
            replyTo = replyTo,
            editedFrom = null,
            timelineGroup = null,
        )

        val note = NoteRecord.fromDto(dto)

        assertEquals(dto.id, note.id)
        assertEquals(dto.content, note.content)
        assertEquals(dto.tags, note.tags)
        assertEquals(dto.color, note.color)
        assertEquals(dto.createdAt, note.createdAt)
        assertEquals(dto.deleted, note.deleted)
        assertEquals(replyTo.id, note.replyTo?.id)
        assertEquals(replyTo.contentPreview, note.replyTo?.contentPreview)
        assertEquals(replyTo.createdAt, note.replyTo?.createdAt)
        assertNull(note.editedFrom)
    }
}
