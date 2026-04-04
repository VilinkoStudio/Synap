package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.NoteDto
import org.junit.Assert.assertEquals
import org.junit.Test

class NoteRecordTest {
    @Test
    fun mapsFromGeneratedDto() {
        val dto = NoteDto(
            id = "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            content = "hello",
            tags = listOf("rust", "android"),
            createdAt = 1234L,
            deleted = true,
        )

        val note = NoteRecord.fromDto(dto)

        assertEquals(dto.id, note.id)
        assertEquals(dto.content, note.content)
        assertEquals(dto.tags, note.tags)
        assertEquals(dto.createdAt, note.createdAt)
        assertEquals(dto.deleted, note.deleted)
    }
}
