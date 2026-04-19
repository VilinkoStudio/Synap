package com.synap.app.widget

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.RemoteViews
import com.synap.app.R

class QuickNoteWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        for (appWidgetId in appWidgetIds) {
            val views = RemoteViews(context.packageName, R.layout.widget_quick_note)

            // 1. 动态获取当前激活的 Launcher 组件
            val launchIntent = context.packageManager.getLaunchIntentForPackage(context.packageName)
            val componentName = launchIntent?.component

            // 2. 构造意图
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse("synap://editor")).apply {
                component = componentName
                // 使用 CLEAR_TASK 保证像冷启动一样拉起全新页面
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            }

            // 3. 构造 PendingIntent
            val pendingIntent = PendingIntent.getActivity(
                context,
                appWidgetId,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            // ========== 核心修复：把点击事件绑在外层容器上 ==========
            views.setOnClickPendingIntent(R.id.widget_root, pendingIntent)

            appWidgetManager.updateAppWidget(appWidgetId, views)
        }
    }
}