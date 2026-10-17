package dev.holocron.app

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import dev.holocron.core.ArchiveSession
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.concurrent.TimeUnit

class SavedArchiveTest {
    @get:Rule val compose = createComposeRule()
    private fun waitText(text: String) { compose.waitUntil(15_000) { compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty() } }
    @Test fun favorite_download_offline_detail_and_reopened_archive() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = File(context.cacheDir, "offline-${System.nanoTime()}.sqlite")
        val records = JSONObject(InstrumentationRegistry.getInstrumentation().context.assets.open("catalog.json").bufferedReader().use { it.readText() })
        fun page(kind: String) = JSONObject().put("count",1).put("next",JSONObject.NULL).put("results",JSONArray().put(records.getJSONObject(kind))).toString()
        FixtureServer().use { server ->
            server.responses.add(Reply(200,page("people")))
            server.responses.add(Reply(200,page("planets")))
            lateinit var archive: ArchiveController
            compose.runOnUiThread { archive = ArchiveController(ArchiveSession.withEndpoint(db.path,server.endpoint)) }
            compose.setContent { HolocronTheme { HolocronNavigation(server.endpoint,archive) } }
            waitText("Luke Skywalker")
            compose.onNodeWithText("Luke Skywalker").performScrollTo().performClick()
            waitText("Add favorite")
            compose.onNodeWithText("Add favorite").performScrollTo().performClick()
            waitText("Remove favorite")
            compose.onNodeWithContentDescription("Back to people").performClick()
            compose.onNodeWithText("Saved archive · 1 favorite").performScrollTo().performClick()
            waitText("Offline mode")
            compose.onNodeWithText("Download collections").performScrollTo().performClick()
            compose.onNodeWithText("Download planets").performScrollTo().performClick()
            waitText("Complete · 1 record")
            compose.onNodeWithText("Hide collection downloads").performScrollTo().performClick()
            compose.onNodeWithContentDescription("Offline mode").performScrollTo().performClick()
            compose.onNodeWithText("Tatooine").performScrollTo().performClick()
            waitText("Climate")
            captureScreen("offline-planet-detail")
            compose.onNodeWithContentDescription("Back to planets").performClick()
            compose.onNodeWithText("Favorites", substring = false).performScrollTo().performClick()
            compose.onNodeWithText("Luke Skywalker").performScrollTo().assertIsDisplayed()
            captureScreen("offline-favorites")
            assertEquals("GET /api/people/?page=1 HTTP/1.1",server.requests.poll(2,TimeUnit.SECONDS))
            assertEquals("GET /api/planets/?page=1 HTTP/1.1",server.requests.poll(2,TimeUnit.SECONDS))
            assertTrue(server.requests.isEmpty())
            compose.runOnUiThread { archive.close() }
            // A new native controller and Rust session read the database with no HTTP.
            lateinit var reopened: ArchiveController
            compose.runOnUiThread { reopened=ArchiveController(ArchiveSession.withEndpoint(db.path,server.endpoint)) }
            compose.waitUntil(5_000) { reopened.state.value.offline && reopened.state.value.favorites.size == 1 && reopened.state.value.collections.any { it.complete } }
            compose.runOnUiThread { reopened.close() }
        }
        listOf("", "-wal", "-shm").forEach { File(db.path+it).delete() }
    }
}
