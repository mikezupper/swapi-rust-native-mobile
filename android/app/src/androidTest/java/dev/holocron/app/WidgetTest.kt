package dev.holocron.app

import android.content.Context
import android.content.ContextWrapper
import android.widget.TextView
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import dev.holocron.core.*
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File

class WidgetTest {
    @get:Rule val compose = createComposeRule()
    @Test fun record_link_opens_saved_detail_after_core_restart_without_network() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = File(context.cacheDir,"widget-link-${System.nanoTime()}.sqlite")
        val planet = JSONObject(InstrumentationRegistry.getInstrumentation().context.assets.open("catalog.json").bufferedReader().use { it.readText() }).getJSONObject("planets")
        FixtureServer().use { server ->
            server.responses.add(Reply(200,planet.toString()))
            lateinit var archive: ArchiveController
            lateinit var loader: CatalogController
            compose.runOnUiThread { archive = ArchiveController(ArchiveSession.withEndpoint(db.path,server.endpoint)); loader = CatalogController(archive.session.catalogSession(CatalogCategory.PLANETS,1u)); loader.refresh() }
            compose.waitUntil(10_000) { loader.state.value.items.isNotEmpty() }
            compose.runOnUiThread { archive.session.setOffline(true) }
            compose.waitUntil(5_000) { archive.state.value.offline }
            compose.runOnUiThread { loader.close(); archive.close(); archive = ArchiveController(ArchiveSession.withEndpoint(db.path,server.endpoint)) }
            compose.waitUntil(5_000) { archive.state.value.offline }
            compose.setContent { HolocronTheme { HolocronNavigation(server.endpoint,archive,initialLink = recordLink(CatalogCategory.PLANETS,1u)) } }
            compose.waitUntil(10_000) { compose.onAllNodesWithText("Climate").fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithText("Tatooine").assertIsDisplayed()
            compose.captureContent("widget-linked-detail")
            assertEquals(1,server.requests.size)
            compose.runOnUiThread { archive.close() }
        }
        listOf("", "-wal", "-shm").forEach { File(db.path+it).delete() }
    }
    @Test fun widget_remote_views_render_saved_cards_and_useful_empty_state() {
        val target = InstrumentationRegistry.getInstrumentation().targetContext
        val preferences = "widget-test-${System.nanoTime()}"
        val context = object : ContextWrapper(target) {
            override fun getSharedPreferences(name: String, mode: Int) = super.getSharedPreferences(preferences,mode)
        }
        val cards = JSONArray().put(JSONObject().put("name","Luke Skywalker").put("subtitle","Favorite · People").put("url","holocron://record/people/1"))
            .put(JSONObject().put("name","Tatooine").put("subtitle","Saved discovery · Planets").put("url","holocron://record/planets/1"))
        context.getSharedPreferences("widget",0).edit().putString("cards",cards.toString()).commit()
        var empty by mutableStateOf(false)
        lateinit var rendered: android.view.View
        compose.setContent { HolocronTheme { Box(Modifier.fillMaxSize().padding(24.dp)) { key(empty) { AndroidView(factory = {
            HolocronWidget.views(context).apply(context,null).also { view ->
                rendered = view
                assertEquals(if (empty) "Your galaxy, closer." else "Luke Skywalker",view.findViewById<TextView>(R.id.widget_name).text.toString())
            }
        },modifier = Modifier.fillMaxWidth().height(250.dp)) } } } }
        fun captureWidget(name: String) {
            compose.waitUntil(5_000) { rendered.width > 0 && rendered.height > 0 && rendered.isShown }
            compose.runOnIdle {
                val bitmap = android.graphics.Bitmap.createBitmap(rendered.width,rendered.height,android.graphics.Bitmap.Config.ARGB_8888)
                rendered.draw(android.graphics.Canvas(bitmap))
                val folder = File(target.filesDir,"screenshots").also { it.mkdirs() }
                File(folder,"$name.png").outputStream().use { bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG,100,it) }
                bitmap.recycle()
            }
        }
        compose.waitForIdle(); captureWidget("home-widget")
        compose.runOnUiThread { context.getSharedPreferences("widget",0).edit().clear().commit(); empty = true }
        compose.waitForIdle(); captureWidget("home-widget-empty")
        target.deleteSharedPreferences(preferences)
    }
}
