package dev.holocron.app

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import java.util.concurrent.TimeUnit

class CatalogScreenTest {
    @get:Rule val compose = createComposeRule()
    private val records get() = JSONObject(InstrumentationRegistry.getInstrumentation().context.assets.open("catalog.json").bufferedReader().use { it.readText() })
    private fun page(items: List<JSONObject>, next: String? = null, total: Int = items.size) = JSONObject()
        .put("count", total).put("next", next ?: JSONObject.NULL).put("results", JSONArray(items)).toString()
    private fun waitText(value: String) { compose.waitUntil(15_000) { compose.onAllNodesWithText(value).fetchSemanticsNodes().isNotEmpty() } }
    private fun request(server: FixtureServer, path: String) { assertEquals("GET /api/$path HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS)) }

    @Test fun everyCategoryLoadsItsOwnCollectionAndDetailThroughRust() {
        FixtureServer().use { server ->
            server.responses.add(Reply(200, page(listOf(records.getJSONObject("people")))))
            compose.setContent { HolocronTheme { HolocronNavigation(server.endpoint) } }
            waitText("Luke Skywalker"); request(server, "people/?page=1")
            val cases = listOf(
                Triple("Planets", "Tatooine", "Climate"), Triple("Starships", "Death Star", "Model"),
                Triple("Vehicles", "Sand Crawler", "Model"), Triple("Species", "Wookiee", "Classification"),
                Triple("Films", "A New Hope", "Episode"))
            cases.forEach { (category, name, fact) ->
                val path = category.lowercase(); val record = records.getJSONObject(path)
                val id = record.getString("url").trimEnd('/').substringAfterLast('/')
                server.responses.add(Reply(200, page(listOf(record))))
                server.responses.add(Reply(200, record.toString()))
                compose.onNodeWithText(category).performScrollTo().performClick()
                waitText(name); request(server, "$path/?page=1")
                captureScreen("catalog-$path")
                compose.onNodeWithText(name).performScrollTo().performClick()
                waitText(fact); request(server, "$path/$id/")
                compose.onNodeWithText(fact).performScrollTo().assertIsDisplayed()
                if (path == "films") compose.onNodeWithText("Opening crawl").performScrollTo().assertIsDisplayed()
                captureScreen("catalog-$path-detail")
                compose.onNodeWithContentDescription("Back to $path").performClick()
                waitText(name)
            }
            compose.onNodeWithText("People").performScrollTo().performClick()
            waitText("Luke Skywalker")
            compose.waitForIdle()
        }
    }

    @Test fun planetPageFailureRetrySearchAndBackPreserveCollectionState() {
        FixtureServer().use { server ->
            val planet=records.getJSONObject("planets")
            val second=JSONObject(planet.toString()).put("name","Alderaan").put("url","https://swapi.dev/api/planets/2/")
            server.responses.add(Reply(200,page(listOf(records.getJSONObject("people")))))
            server.responses.add(Reply(200,page(listOf(planet),server.endpoint+"planets/?page=2",2)))
            server.responses.add(Reply(503,""))
            server.responses.add(Reply(200,page(listOf(second),total=2)))
            server.responses.add(Reply(200,page(emptyList())))
            server.responses.add(Reply(200,page(listOf(second))))
            server.responses.add(Reply(200,second.toString()))
            compose.setContent { HolocronTheme { HolocronNavigation(server.endpoint) } }
            waitText("Luke Skywalker"); request(server,"people/?page=1")
            compose.onNodeWithText("Planets").performScrollTo().performClick()
            waitText("Tatooine"); request(server,"planets/?page=1")
            compose.onNodeWithText("Load more planets").performScrollTo().performClick()
            waitText("Try again"); request(server,"planets/?page=2")
            compose.onNodeWithText("Tatooine").performScrollTo().assertIsDisplayed()
            compose.onNodeWithText("Try again").performScrollTo().performClick()
            waitText("Alderaan"); request(server,"planets/?page=2")
            compose.onNodeWithText("Search planets").performScrollTo().performTextInput("zzzz")
            waitText("No signals found"); request(server,"planets/?page=1&search=zzzz")
            compose.onNodeWithText("Search planets").performTextReplacement("Alderaan")
            waitText("Alderaan"); request(server,"planets/?page=1&search=Alderaan")
            compose.onNodeWithText("Search planets").performImeAction()
            compose.onNode(hasText("Alderaan") and hasClickAction() and !hasSetTextAction()).performScrollTo().performClick()
            waitText("Climate"); request(server,"planets/2/")
            compose.onNodeWithContentDescription("Back to planets").performClick()
            compose.onNodeWithText("Search planets").performScrollTo().assertTextContains("Alderaan")
            compose.waitForIdle()
        }
    }
}
