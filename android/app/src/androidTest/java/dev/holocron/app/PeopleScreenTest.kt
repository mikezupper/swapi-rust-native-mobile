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
import java.util.concurrent.CountDownLatch
import dev.holocron.core.PeoplePhase
import dev.holocron.core.PeopleSession

class PeopleScreenTest {
    @get:Rule val compose = createComposeRule()
    private fun person(id: Int, name: String) = JSONObject(InstrumentationRegistry.getInstrumentation().context.assets.open("luke.json").bufferedReader().use { it.readText() })
        .put("url", "https://swapi.dev/api/people/$id/").put("name", name)
    private fun page(server: FixtureServer, people: List<JSONObject>, next: Boolean = false, total: Int = if (next) 3 else people.size) = JSONObject()
        .put("count", total).put("next", if (next) server.endpoint + "people/?page=2" else JSONObject.NULL)
        .put("results", JSONArray(people)).toString()
    private fun waitText(text: String) { compose.waitUntil(10_000) { compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty() } }

    @Test fun disposingPeopleWhileLoadingRejectsQueuedDelivery() {
        FixtureServer().use { server ->
            val release = CountDownLatch(1)
            server.responses.add(Reply(200, page(server, emptyList()), release))
            lateinit var controller: PeopleController
            compose.runOnUiThread {
                controller = PeopleController(PeopleSession.withEndpoint(server.endpoint))
                controller.refresh()
            }
            compose.setContent { HolocronTheme { androidx.compose.material3.Text("Lifetime test host") } }
            compose.waitUntil(5_000) { controller.state.value.phase == PeoplePhase.Loading }
            assertEquals("GET /api/people/?page=1 HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            val before = controller.state.value
            compose.runOnUiThread { controller.close(); controller.close() }
            release.countDown()
            server.completed.await(5, TimeUnit.SECONDS)
            compose.waitForIdle()
            assertEquals(before, controller.state.value)
        }
    }

    @Test fun browseDetailBackPageRetrySearchEmptyAndRefreshThroughRust() {
        FixtureServer().use { server ->
            val luke = person(1, "Luke Skywalker")
            val droid = person(2, "C-3PO")
            val leia = person(5, "Leia Organa")
            server.responses.add(Reply(200, page(server, listOf(luke, droid), true)))
            server.responses.add(Reply(200, droid.toString()))
            server.responses.add(Reply(200, droid.toString())) // detail refresh preserves ID
            server.responses.add(Reply(503, ""))
            server.responses.add(Reply(200, page(server, listOf(leia), total = 3)))
            server.responses.add(Reply(200, page(server, emptyList())))
            server.responses.add(Reply(200, page(server, listOf(leia))))
            server.responses.add(Reply(200, page(server, listOf(leia))))
            compose.setContent { HolocronTheme { HolocronNavigation(server.endpoint) } }
            waitText("C-3PO")
            captureScreen("people-fixture")
            assertEquals("GET /api/people/?page=1 HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            compose.onNodeWithText("C-3PO").performScrollTo().performClick()
            waitText("172 cm")
            captureScreen("detail-fixture")
            compose.onNodeWithText("Refresh record").performScrollTo().performClick()
            waitText("172 cm")
            repeat(2) { assertEquals("GET /api/people/2/ HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS)) }
            compose.onNodeWithContentDescription("Back to people").performClick()
            compose.onNodeWithText("Load more people").performScrollTo().performClick()
            waitText("Try again")
            captureScreen("page-failure-fixture")
            compose.onNodeWithText("C-3PO").performScrollTo().assertIsDisplayed()
            compose.onNodeWithText("Try again").performScrollTo().performClick()
            waitText("Leia Organa")
            repeat(2) { assertEquals("GET /api/people/?page=2 HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS)) }
            compose.onNodeWithText("Search people").performScrollTo().performTextInput("zzzz")
            waitText("No signals found")
            captureScreen("empty-fixture")
            assertEquals("GET /api/people/?page=1&search=zzzz HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            compose.onNodeWithText("Search people").performTextReplacement("Leia")
            waitText("Leia Organa")
            assertEquals("GET /api/people/?page=1&search=Leia HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            compose.onNodeWithText("Search people").performImeAction()
            compose.onNodeWithContentDescription("Refresh people").performScrollTo().performClick()
            assertEquals("GET /api/people/?page=1&search=Leia HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            waitText("Leia Organa")
            compose.onNodeWithText("Leia Organa").performScrollTo().assertIsDisplayed()
        }
    }
}
