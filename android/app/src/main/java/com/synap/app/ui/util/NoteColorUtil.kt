package com.synap.app.ui.util

import androidx.compose.ui.graphics.Color

/**
 * UI helpers for note color display.
 *
 * Wire encoding of `$color(...)` / legacy `$RRGGBB` lives in the Rust core.
 * Android only receives `NoteRecord.color` (`#rrggbb`) and sends color via
 * create/edit/setNoteColor APIs — never by stuffing color into tags.
 */
object NoteColorUtil {

    private val PRESET_HUES = listOf(
        0f to "红",
        30f to "橙",
        55f to "黄",
        130f to "绿",
        210f to "蓝",
        270f to "紫",
    )

    fun hueToColor(hue: Float): Color = Color.hsv(hue, 1f, 1f)

    fun hueToCssHex(hue: Float): String {
        val color = hueToColor(hue)
        val r = (color.red * 255).toInt()
        val g = (color.green * 255).toInt()
        val b = (color.blue * 255).toInt()
        return "#%02x%02x%02x".format(r, g, b)
    }

    fun parseCssHex(color: String?): Color? {
        val trimmed = color?.trim().orEmpty()
        if (trimmed.isEmpty()) return null
        return try {
            Color(android.graphics.Color.parseColor(if (trimmed.startsWith("#")) trimmed else "#$trimmed"))
        } catch (_: Exception) {
            null
        }
    }

    fun extractHueFromCss(color: String?): Float? {
        val parsed = parseCssHex(color) ?: return null
        val hsv = FloatArray(3)
        android.graphics.Color.RGBToHSV(
            (parsed.red * 255).toInt(),
            (parsed.green * 255).toInt(),
            (parsed.blue * 255).toInt(),
            hsv,
        )
        return hsv[0]
    }

    fun hueToDisplayName(hue: Float): String {
        for ((presetHue, name) in PRESET_HUES) {
            if (kotlin.math.abs(hue - presetHue) <= 5f) return name
        }
        return hue.toInt().toString()
    }

    fun colorCssToDisplayName(color: String?): String? {
        val hue = extractHueFromCss(color) ?: return null
        return hueToDisplayName(hue)
    }
}
