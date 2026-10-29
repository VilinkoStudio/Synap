package com.synap.app

import android.content.Context
import android.content.res.Configuration
import android.os.LocaleList
import com.synap.app.ui.data.sampleLanguages
import java.util.Locale

object LocaleHelper {
    private const val PREFS = "synap_settings"
    private const val KEY = "selectedLanguage"
    private var systemLocale: Locale? = null

    fun captureSystemLocale() {
        if (systemLocale == null) systemLocale = Locale.getDefault()
    }

    fun wrap(base: Context): Context {
        val index = base.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getInt(KEY, 0)
        val tag = if (index > 0) sampleLanguages().getOrNull(index - 1)?.tag else null
        val locale = if (tag != null) Locale.forLanguageTag(tag) else systemLocale ?: return base
        Locale.setDefault(locale)

        val configuration = Configuration(base.resources.configuration).apply {
            setLocale(locale)
            setLocales(LocaleList(locale))
            setLayoutDirection(locale)
        }
        return base.createConfigurationContext(configuration)
    }
}
