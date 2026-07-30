package com.synap.app.data.model

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SearchResultDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SearchMatchRangeDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SearchSourceDto
import com.fuwaki.synap.bindings.uniffi.synap_coreffi.SearchTextMatchDto

enum class SearchSourceRecord {
    Fuzzy,
    Semantic,
}

enum class SearchTextMatchRecord {
    Contiguous,
    Fuzzy,
}

data class SearchMatchRange(
    val start: UInt,
    val end: UInt,
) {
    companion object {
        fun fromDto(dto: SearchMatchRangeDto): SearchMatchRange = SearchMatchRange(
            start = dto.start,
            end = dto.end,
        )
    }
}

data class SearchResultRecord(
    val note: NoteRecord,
    val score: Float,
    val sources: List<SearchSourceRecord>,
    val textMatch: SearchTextMatchRecord?,
    val textMatchRanges: List<SearchMatchRange>?,
) {
    companion object {
        fun fromDto(dto: SearchResultDto): SearchResultRecord = SearchResultRecord(
            note = NoteRecord.fromDto(dto.note),
            score = dto.score,
            sources = dto.sources.map(SearchSourceDto::toSearchSourceRecord),
            textMatch = dto.textMatch?.toSearchTextMatchRecord(),
            textMatchRanges = dto.textMatchRanges?.map(SearchMatchRange::fromDto),
        )
    }
}

internal fun SearchSourceDto.toSearchSourceRecord(): SearchSourceRecord =
    when (this) {
        SearchSourceDto.FUZZY -> SearchSourceRecord.Fuzzy
        SearchSourceDto.SEMANTIC -> SearchSourceRecord.Semantic
    }

internal fun SearchTextMatchDto.toSearchTextMatchRecord(): SearchTextMatchRecord =
    when (this) {
        SearchTextMatchDto.CONTIGUOUS -> SearchTextMatchRecord.Contiguous
        SearchTextMatchDto.FUZZY -> SearchTextMatchRecord.Fuzzy
    }

internal fun SearchResultDto.toSearchResultRecord(): SearchResultRecord =
    SearchResultRecord.fromDto(this)

internal fun List<SearchResultDto>.toSearchResultRecords(): List<SearchResultRecord> =
    map(SearchResultRecord::fromDto)
