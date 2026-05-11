package com.synap.app.ui.util

import androidx.compose.ui.graphics.Color

object NoteColorUtil {

    private val COLOR_TAG_REGEX = Regex("""^\$([0-9a-fA-F]{6})$""")

    fun isColorTag(tag: String): Boolean = COLOR_TAG_REGEX.matches(tag)

    fun parseNoteColor(tags: List<String>): Color? {
        for (tag in tags) {
            val match = COLOR_TAG_REGEX.matchEntire(tag)
            if (match != null) {
                val hex = match.groupValues[1]
                return try {
                    Color(android.graphics.Color.parseColor("#$hex"))
                } catch (_: Exception) {
                    null
                }
            }
        }
        return null
    }

    fun extractColorHue(tags: List<String>): Float? {
        val color = parseNoteColor(tags) ?: return null
        val hsv = FloatArray(3)
        android.graphics.Color.RGBToHSV(
            (color.red * 255).toInt(),
            (color.green * 255).toInt(),
            (color.blue * 255).toInt(),
            hsv
        )
        return hsv[0]
    }

    fun colorToTag(color: Color): String {
        val r = (color.red * 255).toInt()
        val g = (color.green * 255).toInt()
        val b = (color.blue * 255).toInt()
        return "\$%02x%02x%02x".format(r, g, b)
    }

    fun hueToColor(hue: Float): Color = Color.hsv(hue, 1f, 1f)

    private val PRESET_HUES = listOf(
        0f to "红",
        30f to "橙",
        55f to "黄",
        130f to "绿",
        210f to "蓝",
        270f to "紫",
    )

    fun hueToDisplayName(hue: Float): String {
        for ((presetHue, name) in PRESET_HUES) {
            if (kotlin.math.abs(hue - presetHue) <= 5f) return name
        }
        return hue.toInt().toString()
    }

    fun colorTagToDisplayName(tag: String): String? {
        val color = parseNoteColor(listOf(tag)) ?: return null
        val hsv = FloatArray(3)
        android.graphics.Color.RGBToHSV(
            (color.red * 255).toInt(),
            (color.green * 255).toInt(),
            (color.blue * 255).toInt(),
            hsv
        )
        return hueToDisplayName(hsv[0])
    }

    fun filterDisplayTags(tags: List<String>): List<String> {
        return tags
            .filter { !COLOR_TAG_REGEX.matches(it) }
            .map { tag -> if (tag.startsWith("$$")) tag.substring(1) else tag }
    }

    fun prepareStorageTags(displayTags: List<String>, colorHue: Float?): List<String> {
        val result = displayTags.map { tag ->
            if (tag.startsWith("$")) "$$tag" else tag
        }.toMutableList()
        if (colorHue != null) {
            result.add(colorToTag(hueToColor(colorHue)))
        }
        return result
    }
}
