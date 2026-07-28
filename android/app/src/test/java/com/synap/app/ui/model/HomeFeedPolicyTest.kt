package com.synap.app.ui.model

import com.synap.app.data.model.NoteDraftRecord
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeFeedPolicyTest {
    @Test
    fun draftMappingPreservesPresentationAndRelations() {
        val reply = NoteBrief("parent", "parent preview", 10L)
        val edited = NoteBrief("origin", "origin preview", 20L)
        val draft = NoteDraftRecord(
            id = "draft-1",
            content = "# Draft",
            tags = listOf("work"),
            color = "#123456",
            replyTo = reply.id,
            editedFrom = edited.id,
            createdAt = 100L,
            updatedAt = 200L,
            persisted = true,
            revision = 3uL,
        ).toUiNote(replyTo = reply, editedFrom = edited)

        assertTrue(draft.isDraft)
        assertEquals("draft-1", draft.draftId)
        assertEquals("# Draft", draft.content)
        assertEquals(listOf("work"), draft.tags)
        assertEquals("#123456", draft.color)
        assertEquals(200L, draft.timestamp)
        assertEquals(reply, draft.replyTo)
        assertEquals(edited, draft.editedFrom)
    }

    @Test
    fun homeFeedMergesDraftsByUpdatedTime() {
        val committed = Note(id = "note", content = "note", tags = emptyList(), timestamp = 100L)
        val draft = Note(id = "draft", content = "draft", tags = emptyList(), timestamp = 200L, draftId = "draft")

        assertEquals(listOf("draft", "note"), mergeHomeFeedNotes(listOf(committed), listOf(draft)).map(Note::id))
    }

    @Test
    fun draftUsesTheSameTagFilterAsNotes() {
        val taggedDraft = Note(id = "draft", content = "", tags = listOf("work"), timestamp = 1L, draftId = "draft")
        val untaggedDraft = taggedDraft.copy(id = "untagged", tags = emptyList(), draftId = "untagged")

        assertTrue(taggedDraft.matchesHomeTagFilter(true, emptySet(), false))
        assertFalse(taggedDraft.matchesHomeTagFilter(true, setOf("work"), false))
        assertTrue(untaggedDraft.matchesHomeTagFilter(true, setOf("work"), false))
        assertFalse(untaggedDraft.matchesHomeTagFilter(true, setOf("work"), true))
    }

    @Test
    fun exportSelectionExcludesDraftIds() {
        val note = Note(id = "note", content = "", tags = emptyList(), timestamp = 1L)
        val draft = Note(id = "draft", content = "", tags = emptyList(), timestamp = 2L, draftId = "draft")

        assertEquals(setOf("note"), committedSelectedNoteIds(listOf(note, draft), setOf("note", "draft")))
    }
}
