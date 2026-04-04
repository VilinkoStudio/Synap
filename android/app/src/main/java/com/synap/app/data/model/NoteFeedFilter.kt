package com.synap.app.data.model

enum class NoteFeedStatus {
    All,
    Normal,
    Deleted,
}

enum class TimelineDirection {
    Older,
    Newer,
}

data class NoteFeedFilter(
    val selectedTags: List<String> = emptyList(),
    val includeUntagged: Boolean = true,
    val tagFilterEnabled: Boolean = false,
    val status: NoteFeedStatus = NoteFeedStatus.All,
)
