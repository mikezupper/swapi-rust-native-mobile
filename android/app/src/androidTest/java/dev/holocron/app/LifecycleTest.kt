package dev.holocron.app

import android.content.Intent
import android.os.ParcelFileDescriptor
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.junit4.accessibility.enableAccessibilityChecks
import androidx.lifecycle.Lifecycle
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.concurrent.TimeUnit

@OptIn(ExperimentalTestApi::class)
class LifecycleTest {
    @get:Rule val compose = createEmptyComposeRule()

    @Test fun queryAndContentSurviveBackgroundingAndActivityRecreation() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val source = instrumentation.context.assets.open("luke.json").bufferedReader().use { it.readText() }
        val person = JSONObject(source).put("name", "Leia Organa").put("url", "https://swapi.dev/api/people/5/")
        val page = JSONObject().put("count", 1).put("next", JSONObject.NULL).put("results", JSONArray().put(person)).toString()
        FixtureServer().use { server ->
            server.responses.add(Reply(200, page))
            server.responses.add(Reply(200, page))
            compose.enableAccessibilityChecks()
            ActivityScenario.launch<MainActivity>(Intent(context, MainActivity::class.java).putExtra("endpoint", server.endpoint)).use { activity ->
                compose.waitUntil(10_000) { compose.onAllNodesWithText("Leia Organa").fetchSemanticsNodes().isNotEmpty() }
                assertEquals("Initial load", "GET /api/people/?page=1 HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
                compose.onRoot().tryPerformAccessibilityChecks()
                assertEquals("Audit must not reload", null, server.requests.poll(100, TimeUnit.MILLISECONDS))
                compose.onNodeWithText("Search people").performTextInput("Leia")
                assertEquals("GET /api/people/?page=1&search=Leia HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
                compose.onNode(hasSetTextAction()).performImeAction()
                activity.moveToState(Lifecycle.State.CREATED)
                activity.moveToState(Lifecycle.State.RESUMED)
                activity.recreate()
                compose.onNode(hasSetTextAction()).assertTextContains("Leia")
                compose.onNodeWithText("Leia Organa").assertIsDisplayed()
                compose.onRoot().tryPerformAccessibilityChecks()
                captureScreen("recreated-query-fixture")
                // The retained ViewModel must not create a duplicate request on recreation.
                assertEquals(null, server.requests.poll(100, TimeUnit.MILLISECONDS))
            }
        }
    }

    @Test fun collectWarmScrollFrameStatistics() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val source = instrumentation.context.assets.open("luke.json").bufferedReader().use { it.readText() }
        val records = JSONArray((1..40).map { id -> JSONObject(source).put("name", "Explorer $id").put("url", "https://swapi.dev/api/people/$id/") })
        val page = JSONObject().put("count", 40).put("next", JSONObject.NULL).put("results", records).toString()
        FixtureServer().use { server ->
            server.responses.add(Reply(200, page))
            ActivityScenario.launch<MainActivity>(Intent(context, MainActivity::class.java).putExtra("endpoint", server.endpoint)).use {
                compose.waitUntil(10_000) { compose.onAllNodesWithText("Explorer 1").fetchSemanticsNodes().isNotEmpty() }
                val scroll = compose.onNode(hasScrollToIndexAction())
                scroll.performTouchInput { swipeUp() }
                shell("dumpsys gfxinfo dev.holocron.app reset")
                repeat(8) { scroll.performTouchInput { swipeUp() } }
                repeat(8) { scroll.performTouchInput { swipeDown() } }
                // Finish the final fling using Compose's test clock before
                // ActivityScenario waits for Android's main looper to become idle.
                compose.waitForIdle()
                val directory = File(context.filesDir, "screenshots").also { it.mkdirs() }
                File(directory, "scroll-gfxinfo.txt").writeText(shell("dumpsys gfxinfo dev.holocron.app framestats"))
                File(directory, "display.txt").writeText(shell("dumpsys display"))
                captureScreen("scroll-fixture")
            }
        }
    }

    private fun shell(command: String): String {
        val descriptor = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(command)
        return ParcelFileDescriptor.AutoCloseInputStream(descriptor).bufferedReader().use { it.readText() }
    }
}
