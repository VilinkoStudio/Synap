package com.synap.app.ui.model

import com.synap.app.data.model.NoteRecord
import com.synap.app.data.model.ReplyItem

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
)

fun ReplyItem.toUiNote(parentSummary: String? = null): Note =
    note.toUiNote(parentSummary = parentSummary)
