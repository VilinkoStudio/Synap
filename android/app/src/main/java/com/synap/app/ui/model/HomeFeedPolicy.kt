package com.synap.app.ui.model

fun mergeHomeFeedNotes(committed: List<Note>, drafts: List<Note>): List<Note> =
    (committed + drafts).sortedByDescending(Note::timestamp)

fun Note.matchesHomeTagFilter(
    showTagBar: Boolean,
    unselectedTags: Set<String>,
    isUntaggedUnselected: Boolean,
): Boolean {
    if (!showTagBar || (unselectedTags.isEmpty() && !isUntaggedUnselected)) {
        return true
    }
    if (tags.isEmpty()) {
        return !isUntaggedUnselected
    }
    return tags.any { it !in unselectedTags }
}

fun committedSelectedNoteIds(notes: List<Note>, selectedIds: Set<String>): Set<String> =
    notes
        .asSequence()
        .filter { it.draftId == null && it.id in selectedIds }
        .map(Note::id)
        .toSet()
