package dev.holocron.app

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.RemoteViews
import dev.holocron.core.SavedEntry
import dev.holocron.core.recordLink
import org.json.JSONArray
import org.json.JSONObject

/** A rendering-only system surface. Rust chooses the saved records; no widget HTTP. */
class HolocronWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) { ids.forEach { manager.updateAppWidget(it, views(context)) } }
    companion object {
        private val writer = java.util.concurrent.Executors.newSingleThreadExecutor()
        fun publish(context: Context, records: List<SavedEntry>) {
            val application = context.applicationContext
            val json = JSONArray(records.map { saved -> JSONObject().put("name", saved.entry.name)
                .put("subtitle", "${if (saved.favorite) "Favorite" else "Saved discovery"} · ${saved.category.title}")
                .put("url", recordLink(saved.category, saved.entry.id)) }).toString()
            writer.execute {
                val prefs = application.getSharedPreferences("widget", Context.MODE_PRIVATE)
                if (prefs.getString("cards", null) != json && prefs.edit().putString("cards", json).commit()) {
                    val manager = AppWidgetManager.getInstance(application)
                    manager.getAppWidgetIds(ComponentName(application, HolocronWidget::class.java)).forEach { manager.updateAppWidget(it, views(application)) }
                }
            }
        }
        fun views(context: Context): RemoteViews {
            val cards = runCatching { JSONArray(context.getSharedPreferences("widget", Context.MODE_PRIVATE).getString("cards", "[]")) }.getOrElse { JSONArray() }
            val views = RemoteViews(context.packageName, R.layout.holocron_widget)
            fun launch(url: String?): PendingIntent {
                val intent = Intent(context, MainActivity::class.java).apply { action = Intent.ACTION_VIEW; if (url != null) data = Uri.parse(url); flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP }
                return PendingIntent.getActivity(context, 0, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
            }
            val first = cards.optJSONObject(0)
            views.setOnClickPendingIntent(R.id.widget_root, launch(first?.optString("url")))
            if (first != null) { views.setTextViewText(R.id.widget_name, first.optString("name")); views.setTextViewText(R.id.widget_summary, first.optString("subtitle")) }
            val second = cards.optJSONObject(1)
            views.setTextViewText(R.id.widget_second, second?.let { "Discover ${it.optString("name")} →" } ?: "Open your saved galaxy →")
            views.setOnClickPendingIntent(R.id.widget_second, launch(second?.optString("url")))
            return views
        }
        fun pin(context: Context) {
            val manager = AppWidgetManager.getInstance(context)
            if (manager.isRequestPinAppWidgetSupported) manager.requestPinAppWidget(ComponentName(context, HolocronWidget::class.java), null, null)
            else android.widget.Toast.makeText(context, "Touch and hold your home screen, choose Widgets, then Holocron.", android.widget.Toast.LENGTH_LONG).show()
        }
    }
}
