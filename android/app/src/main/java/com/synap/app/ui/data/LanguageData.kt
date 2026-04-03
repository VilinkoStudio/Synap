package com.synap.app.ui.data
data class AppLocale(val displayName: String, val tag: String)

fun sampleLanguages(): List<AppLocale> = listOf(
    AppLocale("简体中文", "zh-CN"),
    AppLocale("繁体中文（适用于中国台湾）", "zh-TW"),
    AppLocale("繁体中文（适用于中国香港特别行政区）", "zh-HK"),
    AppLocale("English", "en"),
    AppLocale("日本語", "ja"),
    AppLocale("한국어", "ko"),
)
