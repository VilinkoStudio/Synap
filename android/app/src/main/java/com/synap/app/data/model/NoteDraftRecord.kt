package com.synap.app.data.model

data class NoteDraftRecord(
    val id: String,
    val content: String,
    val tags: List<String>,
    val color: String?,
    val replyTo: String?,
    val editedFrom: String?,
    val createdAt: Long,
    val updatedAt: Long,
    val persisted: Boolean,
    val revision: ULong,
)
