package com.synap.app.ui.util

data class ParsedTable(
    val startOffset: Int,
    val endOffset: Int,
    val headers: List<String>,
    val rows: List<List<String>>,
) {
    val colCount: Int get() = headers.size
    val rowCount: Int get() = rows.size

    fun toMarkdown(): String = buildString {
        append("| ")
        append(headers.joinToString(" | "))
        append(" |\n")
        append("| ")
        repeat(colCount) { append("--- | ") }
        append("\n")
        for (row in rows) {
            append("| ")
            // 补齐列数
            val padded = row.toMutableList()
            while (padded.size < colCount) padded.add("")
            append(padded.take(colCount).joinToString(" | "))
            append(" |\n")
        }
    }
}

object MarkdownTableParser {

    private val tableLineRegex = Regex("^\\|(.+)\\|\\s*$")
    private val separatorRegex = Regex("^\\|([\\s:]*---[\\s:]*(?:\\|[\\s:]*---[\\s:]*)*)\\|\\s*$")

    fun parseTables(text: String): List<ParsedTable> {
        val tables = mutableListOf<ParsedTable>()
        val lines = text.split("\n")
        var i = 0

        while (i < lines.size) {
            val tableResult = tryParseTable(lines, i)
            if (tableResult != null) {
                val (table, lineCount) = tableResult
                val startOffset = lines.take(i).sumOf { it.length + 1 }.coerceAtMost(text.length)
                val endOffset = lines.take(i + lineCount).sumOf { it.length + 1 }.coerceAtMost(text.length)
                tables.add(table.copy(startOffset = startOffset, endOffset = endOffset))
                i += lineCount
            } else {
                i++
            }
        }

        return tables
    }

    private fun tryParseTable(lines: List<String>, startIndex: Int): Pair<ParsedTable, Int>? {
        // 至少需要3行：表头、分隔符、至少一行数据
        if (startIndex + 2 >= lines.size) return null

        // 检查第一行是否是表头
        val headerLine = lines[startIndex]
        val headerMatch = tableLineRegex.find(headerLine) ?: return null

        // 检查第二行是否是分隔符
        val separatorLine = lines[startIndex + 1]
        if (!separatorRegex.matches(separatorLine)) return null

        // 解析表头
        val headers = parseRow(headerMatch.groupValues[1])

        // 解析数据行
        val rows = mutableListOf<List<String>>()
        var lineCount = 2 // 表头 + 分隔符

        var j = startIndex + 2
        while (j < lines.size) {
            val line = lines[j]
            val match = tableLineRegex.find(line) ?: break
            rows.add(parseRow(match.groupValues[1]))
            lineCount++
            j++
        }

        if (rows.isEmpty()) return null

        return Pair(
            ParsedTable(startOffset = 0, endOffset = 0, headers = headers, rows = rows),
            lineCount
        )
    }

    private fun parseRow(content: String): List<String> {
        return content.split("|").map { it.trim() }
    }
}
