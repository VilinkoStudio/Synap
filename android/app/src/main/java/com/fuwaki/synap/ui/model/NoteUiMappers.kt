package com.fuwaki.synap.ui.model

import com.fuwaki.synap.data.model.NoteRecord
import com.fuwaki.synap.data.model.ReplyItem

fun NoteRecord.toUiNote(
    isDeleted: Boolean = false,
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
