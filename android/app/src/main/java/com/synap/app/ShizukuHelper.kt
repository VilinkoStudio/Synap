package com.synap.app

import android.content.ComponentName
import android.content.Context
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.IBinder
import rikka.shizuku.Shizuku

object ShizukuHelper {

    fun isShizukuRunning(): Boolean {
        return runCatching { Shizuku.pingBinder() }.getOrDefault(false)
    }

    fun isShizukuPermissionGranted(): Boolean {
        return runCatching { Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED }
            .getOrDefault(false)
    }

    fun checkPermission(callback: (Boolean) -> Unit) {
        if (!isShizukuRunning()) {
            callback(false)
            return
        }
        if (isShizukuPermissionGranted()) {
            callback(true)
        } else {
            val listener = object : Shizuku.OnRequestPermissionResultListener {
                override fun onRequestPermissionResult(requestCode: Int, grantResult: Int) {
                    Shizuku.removeRequestPermissionResultListener(this)
                    callback(grantResult == PackageManager.PERMISSION_GRANTED)
                }
            }
            Shizuku.addRequestPermissionResultListener(listener)
            Shizuku.requestPermission(1001)
        }
    }

    fun grantWriteSecureSettings(context: Context, onResult: (Boolean) -> Unit) {
        if (!isShizukuPermissionGranted()) {
            onResult(false)
            return
        }
        executeCommand(
            context,
            arrayOf("pm", "grant", context.packageName, "android.permission.WRITE_SECURE_SETTINGS"),
            onResult
        )
    }

    fun setAccessibilityShortcut(context: Context, onResult: (Boolean) -> Unit) {
        if (!isShizukuPermissionGranted()) {
            onResult(false)
            return
        }
        val serviceName = "${context.packageName}/com.synap.app.ShortcutTriggerActivity"
        executeCommand(
            context,
            arrayOf("settings", "put", "secure", "accessibility_shortcut_target_service", serviceName)
        ) { success ->
            if (!success) {
                onResult(false)
                return@executeCommand
            }
            executeCommand(
                context,
                arrayOf("settings", "put", "secure", "accessibility_shortcut_enabled", "1"),
                onResult
            )
        }
    }

    private fun executeCommand(context: Context, command: Array<String>, onResult: (Boolean) -> Unit) {
        val componentName = ComponentName(context, ShellService::class.java)
        val userServiceArgs = Shizuku.UserServiceArgs(componentName)
            .daemon(false)
            .version(1)
            .processNameSuffix("shell")

        var connection: ServiceConnection? = null
        connection = object : ServiceConnection {
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                try {
                    val shellService = IShellService.Stub.asInterface(binder)
                    val exitCode = shellService.exec(command)
                    onResult(exitCode == 0)
                } catch (e: Exception) {
                    onResult(false)
                } finally {
                    android.os.Handler(android.os.Looper.getMainLooper()).post {
                        Shizuku.unbindUserService(userServiceArgs, connection!!, true)
                    }
                }
            }

            override fun onServiceDisconnected(name: ComponentName) {
                onResult(false)
            }
        }

        android.os.Handler(android.os.Looper.getMainLooper()).post {
            Shizuku.bindUserService(userServiceArgs, connection)
        }
    }

    class ShellService : IShellService.Stub() {
        override fun exec(command: Array<String>): Int {
            return try {
                val process = Runtime.getRuntime().exec(command)
                process.waitFor()
            } catch (e: Exception) {
                -1
            }
        }

        override fun destroy() {
            System.exit(0)
        }
    }
}
