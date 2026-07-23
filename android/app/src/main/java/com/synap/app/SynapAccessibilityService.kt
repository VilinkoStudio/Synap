package com.synap.app

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.app.PendingIntent
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.util.Log
import android.view.KeyEvent
import android.view.accessibility.AccessibilityEvent
import android.widget.Toast

class SynapAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "SynapA11y"
        const val KEY_COMBO_WINDOW_MS = 500L
    }

    private var volumeUpPressTime = 0L
    private var volumeDownPressTime = 0L

    override fun onServiceConnected() {
        super.onServiceConnected()

        serviceInfo = serviceInfo.apply {
            eventTypes = AccessibilityEvent.TYPES_ALL_MASK
            feedbackType = AccessibilityServiceInfo.FEEDBACK_GENERIC
            flags = AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS
            notificationTimeout = 100
        }

        Log.d(TAG, "Service connected, key event filter active")
    }

    override fun onKeyEvent(event: KeyEvent?): Boolean {
        if (event == null || event.action != KeyEvent.ACTION_DOWN) return false

        val now = System.currentTimeMillis()

        when (event.keyCode) {
            KeyEvent.KEYCODE_VOLUME_UP -> {
                volumeUpPressTime = now
                Log.d(TAG, "Volume UP (time=$now)")
            }
            KeyEvent.KEYCODE_VOLUME_DOWN -> {
                volumeDownPressTime = now
                Log.d(TAG, "Volume DOWN (time=$now)")
            }
            else -> return false
        }

        if (volumeUpPressTime != 0L && volumeDownPressTime != 0L &&
            Math.abs(volumeUpPressTime - volumeDownPressTime) <= KEY_COMBO_WINDOW_MS
        ) {
            Log.d(TAG, "Combo detected!")
            volumeUpPressTime = 0L
            volumeDownPressTime = 0L
            openNoteEditor()
            return true
        }

        return false
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {}

    override fun onInterrupt() {}

    private fun openNoteEditor() {
        try {
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse("synap://editor")).apply {
                setPackage(packageName)
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                PendingIntent.getActivity(
                    this, 0, intent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                ).send()
            } else {
                @Suppress("DEPRECATION")
                startActivity(intent)
            }
            Log.d(TAG, "Editor launched")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to launch editor", e)
        }
    }
}
