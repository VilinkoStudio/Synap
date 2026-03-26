package com.fuwaki.synap.ui.util

import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

private fun normalizeEpochMillis(timestamp: Long): Long =
    if (timestamp in 0 until 10_000_000_000L) {
        timestamp * 1000
    } else {
        timestamp
    }

fun formatNoteTime(timestamp: Long): String {
    val normalizedTimestamp = normalizeEpochMillis(timestamp)
    val diff = System.currentTimeMillis() - normalizedTimestamp
    val minutes = diff / (60 * 1000)
    val hours = diff / (60 * 60 * 1000)
    val cal = Calendar.getInstance()
    cal.timeInMillis = System.currentTimeMillis()
    val currentYear = cal.get(Calendar.YEAR)
    cal.timeInMillis = normalizedTimestamp
    val targetYear = cal.get(Calendar.YEAR)

    return when {
        currentYear != targetYear -> SimpleDateFormat("yyyy年M月d日", Locale.getDefault()).format(Date(normalizedTimestamp))
        hours < 1 -> if (minutes <= 0) "刚刚" else "${minutes}分钟前"
        hours < 24 -> "${hours}小时前"
        hours < 48 -> "昨天"
        hours < 72 -> "前天"
        else -> SimpleDateFormat("M月d日", Locale.getDefault()).format(Date(normalizedTimestamp))
    }
}
