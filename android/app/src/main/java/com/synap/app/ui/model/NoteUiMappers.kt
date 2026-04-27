package com.synap.app.ui.model

import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.NoteBriefRecord
import com.synap.app.data.model.NoteVersionRecord
import com.synap.app.data.model.ReplyItem

fun NoteBriefRecord.toUiNoteBrief(): NoteBrief = NoteBrief(
    id = id,
    contentPreview = contentPreview,
    createdAt = createdAt,
)

fun NoteRecord.toUiNote(
    isDeleted: Boolean = deleted,
    parentSummary: String? = null,
): Note = Note(
    id = id,
    content = content,
    tags = tags,
    timestamp = createdAt,
    parentSummary = parentSummary,
    isDeleted = isDeleted,
    replyTo = replyTo?.toUiNoteBrief(),
    editedFrom = editedFrom?.toUiNoteBrief(),
)

fun ReplyItem.toUiNote(parentSummary: String? = null): Note =
    note.toUiNote(parentSummary = parentSummary)

fun NoteVersionRecord.toUiNoteVersion(
    parentSummary: String? = null,
): NoteVersion = NoteVersion(
    note = note.toUiNote(parentSummary = parentSummary),
    addedTags = diff.tags.added,
    removedTags = diff.tags.removed,
    diffStats = NoteVersionDiffStats(
        insertedChars = diff.contentStats.insertedChars,
        deletedChars = diff.contentStats.deletedChars,
        insertedLines = diff.contentStats.insertedLines,
        deletedLines = diff.contentStats.deletedLines,
    ),
)
