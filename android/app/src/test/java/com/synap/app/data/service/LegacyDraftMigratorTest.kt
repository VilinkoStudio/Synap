package com.synap.app.data.service

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LegacyDraftMigratorTest {
    @Test
    fun `origin preserves valid edit and reply relationships`() {
        assertEquals(
            LegacyDraftOrigin.Edit("note-1"),
            record(id = "edit", mode = "edit", editNoteId = "note-1").origin(),
        )
        assertEquals(
            LegacyDraftOrigin.Reply("note-2"),
            record(id = "reply", mode = "reply", parentId = "note-2").origin(),
        )
        assertEquals(
            LegacyDraftOrigin.Create,
            record(id = "broken", mode = "edit").origin(),
        )
    }

    @Test
    fun `legacy hue converts to core css color`() {
        assertEquals("#ff0000", record(noteColorHue = 0f).colorCss())
        assertEquals("#00ff00", record(noteColorHue = 120f).colorCss())
        assertEquals("#0000ff", record(noteColorHue = 240f).colorCss())
        assertEquals("#ff00ff", record(noteColorHue = -60f).colorCss())
        assertNull(record().colorCss())
    }

    @Test
    fun `migration deletes only records persisted in core and continues after failure`() = runBlocking {
        val deleted = mutableListOf<String>()
        val persisted = mutableListOf<String>()
        val records = listOf(record(id = "ok"), record(id = "failed"), record(id = "later"))
        val target = object : LegacyDraftImportTarget {
            override suspend fun create(origin: LegacyDraftOrigin): String = "core-${persisted.size}"

            override suspend fun updateAndPersist(draftId: String, legacy: DraftRecord) {
                if (legacy.id == "failed") error("write failed")
                persisted += legacy.id
            }
        }

        migrateLegacyDraftRecords(records, target, deleted::add)

        assertEquals(listOf("ok", "later"), persisted)
        assertEquals(listOf("ok", "later"), deleted)
    }

    @Test
    fun `missing relation falls back to a create draft`() = runBlocking {
        val origins = mutableListOf<LegacyDraftOrigin>()
        val target = object : LegacyDraftImportTarget {
            override suspend fun create(origin: LegacyDraftOrigin): String {
                origins += origin
                if (origin is LegacyDraftOrigin.Edit) error("note not found")
                return "core-id"
            }

            override suspend fun updateAndPersist(draftId: String, legacy: DraftRecord) = Unit
        }

        migrateLegacyDraftRecords(
            listOf(record(mode = "edit", editNoteId = "missing")),
            target,
            onMigrated = {},
        )

        assertEquals(
            listOf(LegacyDraftOrigin.Edit("missing"), LegacyDraftOrigin.Create),
            origins,
        )
    }

    private fun record(
        id: String = "legacy-id",
        mode: String = "create",
        parentId: String? = null,
        editNoteId: String? = null,
        noteColorHue: Float? = null,
    ) = DraftRecord(
        id = id,
        content = "content",
        tags = listOf("tag"),
        noteColorHue = noteColorHue,
        mode = mode,
        parentId = parentId,
        editNoteId = editNoteId,
    )
}
