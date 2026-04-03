package com.synap.app.ui.model

data class Note(
    val id: String,
    val content: String,
    val tags: List<String>,
    val timestamp: Long,
    val parentSummary: String? = null,
    val isDeleted: Boolean = false,
)
