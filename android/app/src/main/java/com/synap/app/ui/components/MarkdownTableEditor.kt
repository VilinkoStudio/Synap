package com.synap.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.ui.util.ParsedTable

@Composable
fun MarkdownTableEditor(
    table: ParsedTable,
    onCellChange: ((rowIndex: Int, colIndex: Int, newValue: String) -> Unit)?,
    modifier: Modifier = Modifier,
) {
    val readOnly = onCellChange == null
    val borderColor = MaterialTheme.colorScheme.outlineVariant
    val headerBg = MaterialTheme.colorScheme.secondaryContainer
    val cellBg = MaterialTheme.colorScheme.surface
    val textStyle = MaterialTheme.typography.bodyMedium
    val cellMinWidth = 100.dp

    BoxWithConstraints(modifier = modifier) {
        val minTableWidth = cellMinWidth * table.colCount
        val needScroll = minTableWidth > maxWidth

        val tableModifier = if (needScroll) {
            Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .border(1.dp, borderColor)
        } else {
            Modifier
                .fillMaxWidth()
                .border(1.dp, borderColor)
        }

        val rowMod = if (needScroll) {
            Modifier.height(IntrinsicSize.Min)
        } else {
            Modifier.fillMaxWidth().height(IntrinsicSize.Min)
        }

        Column(modifier = tableModifier) {
            // 表头 + 分隔线
            Column {
                Row(modifier = rowMod) {
                    for ((colIdx, header) in table.headers.withIndex()) {
                        if (colIdx > 0) {
                            VerticalDivider(color = borderColor)
                        }
                        val cellMod = if (needScroll) {
                            Modifier.requiredWidth(cellMinWidth)
                        } else {
                            Modifier.defaultMinSize(minWidth = cellMinWidth).weight(1f)
                        }
                        TableCell(
                            text = header,
                            isHeader = true,
                            readOnly = readOnly,
                            backgroundColor = headerBg,
                            textStyle = textStyle.copy(fontWeight = FontWeight.Bold),
                            onValueChange = { onCellChange?.invoke(-1, colIdx, it) },
                            modifier = cellMod,
                        )
                    }
                }
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(1.dp)
                        .background(borderColor)
                )
            }

            // 数据行
            for ((rowIdx, row) in table.rows.withIndex()) {
                Column {
                    if (rowIdx > 0) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(1.dp)
                                .background(borderColor)
                        )
                    }
                    Row(modifier = rowMod) {
                        for (colIdx in 0 until table.colCount) {
                            if (colIdx > 0) {
                                VerticalDivider(color = borderColor)
                            }
                            val cellMod = if (needScroll) {
                                Modifier.requiredWidth(cellMinWidth)
                            } else {
                                Modifier.defaultMinSize(minWidth = cellMinWidth).weight(1f)
                            }
                            val cellText = row.getOrElse(colIdx) { "" }
                            TableCell(
                                text = cellText,
                                isHeader = false,
                                readOnly = readOnly,
                                backgroundColor = cellBg,
                                textStyle = textStyle,
                                onValueChange = { onCellChange?.invoke(rowIdx, colIdx, it) },
                                modifier = cellMod,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun TableCell(
    text: String,
    isHeader: Boolean,
    readOnly: Boolean,
    backgroundColor: androidx.compose.ui.graphics.Color,
    textStyle: TextStyle,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    // 存储用：字面量 \n 显示为实际换行
    val displayText = remember(text) { text.replace("\\n", "\n") }
    var cellText by remember(text) { mutableStateOf(displayText) }

    Box(
        modifier = modifier
            .background(backgroundColor)
            .padding(horizontal = 8.dp, vertical = 6.dp),
        contentAlignment = Alignment.CenterStart,
    ) {
        if (readOnly) {
            Text(
                text = cellText.ifEmpty { if (isHeader) "标题" else " " },
                style = textStyle.copy(
                    color = MaterialTheme.colorScheme.onSurface,
                    fontSize = 14.sp,
                    lineHeight = 18.sp,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
        } else {
            BasicTextField(
                value = cellText,
                onValueChange = { newValue ->
                    cellText = newValue
                    // 实际换行存储为字面量 \n
                    onValueChange(newValue.replace("\n", "\\n"))
                },
                textStyle = textStyle.copy(
                    color = MaterialTheme.colorScheme.onSurface,
                    fontSize = 14.sp,
                    lineHeight = 18.sp,
                ),
                cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                singleLine = false,
                maxLines = Int.MAX_VALUE,
                modifier = Modifier.fillMaxWidth(),
            )
            if (cellText.isEmpty()) {
                Text(
                    text = if (isHeader) "标题" else "",
                    style = textStyle.copy(
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f),
                        fontSize = 14.sp,
                    ),
                )
            }
        }
    }
}
