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

            // 1. 动态获取当前激活的 Launcher 组件 (自动识别 Main 或 MainActivityOld)
            val launchIntent = context.packageManager.getLaunchIntentForPackage(context.packageName)
            val componentName = launchIntent?.component

            // 2. 构造指向该组件的 VIEW 动作
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse("synap://editor")).apply {
                component = componentName
                flags = Intent.FLAG_ACTIVITY_NEW_TASK
            }

            // 3. 使用 appWidgetId 作为 requestCode，确保每个小组件的意图都是唯一的
            val pendingIntent = PendingIntent.getActivity(
                context,
                appWidgetId, // 重要：使用唯一 ID 避免意图被系统缓存覆盖
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            views.setOnClickPendingIntent(R.id.widget_btn_add, pendingIntent)
            appWidgetManager.updateAppWidget(appWidgetId, views)
        }
    }
}