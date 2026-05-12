package com.synap.app.data.service

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

data class RelayConfig(
    val baseUrl: String = "",
    val apiKey: String = "",
)

@Singleton
class RelayConfigStore @Inject constructor(
    @ApplicationContext context: Context,
) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun get(): RelayConfig = RelayConfig(
        baseUrl = prefs.getString(KEY_BASE_URL, "").orEmpty(),
        apiKey = prefs.getString(KEY_API_KEY, "").orEmpty(),
    )

    fun save(config: RelayConfig) {
        prefs.edit()
            .putString(KEY_BASE_URL, config.baseUrl.trim())
            .putString(KEY_API_KEY, config.apiKey.trim())
            .apply()
    }

    private companion object {
        const val PREFS_NAME = "relay_config"
        const val KEY_BASE_URL = "base_url"
        const val KEY_API_KEY = "api_key"
    }
}
