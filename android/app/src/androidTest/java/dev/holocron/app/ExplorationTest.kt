package dev.holocron.app

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

class ExplorationTest {
    @get:Rule val compose = createComposeRule()
    private fun waitText(text: String) { compose.waitUntil(15_000) { compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty() } }
    private fun tap(text: String) {
        if (compose.onAllNodesWithText(text).fetchSemanticsNodes().isEmpty()) compose.onNodeWithTag("catalog-list").performScrollToNode(hasText(text))
        compose.onNodeWithText(text).performScrollTo().performClick()
    }
    private fun capture(name: String) { compose.captureContent(name) }
    @Test fun connected_discovery_comparison_crawl_and_offline_widget_route() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = File(context.cacheDir, "exploration-${System.nanoTime()}.sqlite")
        val fixtures = JSONObject(InstrumentationRegistry.getInstrumentation().context.assets.open("catalog.json").bufferedReader().use { it.readText() })
        fun page(vararg records: JSONObject) = JSONObject().put("count",records.size).put("next",JSONObject.NULL).put("results",JSONArray(records.toList())).toString()
        FixtureServer().use { server ->
            val second = JSONObject(fixtures.getJSONObject("starships").toString()).put("name","Test courier").put("url","https://swapi.dev/api/starships/10/").put("length","30").put("crew","2-4").put("cargo_capacity","unknown").put("hyperdrive_rating","0.5")
            server.responses.add(Reply(200,page(fixtures.getJSONObject("people"))))
            server.responses.add(Reply(200,fixtures.getJSONObject("planets").toString()))
            server.responses.add(Reply(200,page(fixtures.getJSONObject("starships"),second)))
            server.responses.add(Reply(200,page(fixtures.getJSONObject("films"))))
            lateinit var archive: ArchiveController
            compose.runOnUiThread { archive = ArchiveController(ArchiveSession.withEndpoint(db.path,server.endpoint)) }
            compose.setContent { HolocronTheme { HolocronNavigation(server.endpoint,archive) } }
            waitText("Luke Skywalker"); tap("Luke Skywalker")
            waitText("Homeworld · 1"); tap("Homeworld · 1")
            waitText("Climate"); capture("connected-homeworld")
            compose.onNodeWithContentDescription("Back to planets").performClick()
            compose.onNodeWithContentDescription("Back to people").performClick()
            tap("Starships"); waitText("Death Star")
            compose.onNodeWithContentDescription("Compare ship 9").performScrollTo().performClick()
            compose.waitUntil(5_000) { archive.state.value.exploration.ships.size == 1 }
            compose.onNodeWithTag("catalog-list").performScrollToNode(hasContentDescription("Compare ship 10"))
            compose.onNodeWithContentDescription("Compare ship 10").performScrollTo().performClick()
            compose.waitUntil(5_000) { archive.state.value.exploration.ships.size == 2 }
            capture("comparison-tray")
            compose.onNodeWithText("Compare starships").performClick()
            waitText("Starship comparison"); capture("ship-comparison")
            tap("Cargo (kg)")
            compose.onNodeWithText("Test courier: Unknown").performScrollTo().assertIsDisplayed()
            compose.onNodeWithContentDescription("Back").performClick()
            compose.onNodeWithContentDescription("Remove Death Star").performClick()
            compose.waitUntil(5_000) { archive.state.value.exploration.ships.size == 1 }
            compose.waitForIdle()
            compose.onNodeWithContentDescription("Remove Test courier").performClick()
            compose.waitUntil(5_000) { archive.state.value.exploration.ships.isEmpty() }
            compose.waitForIdle()
            tap("Films"); waitText("A New Hope"); tap("A New Hope")
            waitText("Cinematic opening crawl"); tap("Cinematic opening crawl")
            compose.waitForIdle()
            compose.onNodeWithText("Play crawl").performScrollTo()
            compose.mainClock.autoAdvance = false
            compose.onNodeWithText("Play crawl").performClick()
            compose.mainClock.advanceTimeBy(12_000)
            tap("Pause crawl")
            compose.mainClock.autoAdvance = true
            compose.waitForIdle(); capture("film-crawl")
            tap("Read opening crawl")
            compose.onNodeWithText(fixtures.getJSONObject("films").getString("opening_crawl").replace("\r\n","\n")).performScrollTo().assertIsDisplayed()
            compose.onNodeWithContentDescription("Back to films").performClick()
            tap("✦ Discover your galaxy"); waitText("FEATURED WORLD"); capture("discovery")
            compose.onNodeWithText("Recently explored").performScrollTo().assertIsDisplayed()
            compose.runOnUiThread { archive.session.setOffline(true) }
            compose.waitUntil(5_000) { archive.state.value.offline }
            assertEquals("holocron://record/planets/1",recordLink(CatalogCategory.PLANETS,1u))
            assertEquals(1u,parseRecordLink("holocron://record/planets/1")?.id)
            assertTrue(archive.state.value.exploration.recent.isNotEmpty())
            assertTrue(archive.state.value.exploration.widget.isNotEmpty())
            compose.runOnUiThread { archive.close() }
        }
        listOf("", "-wal", "-shm").forEach { File(db.path+it).delete() }
    }
}
