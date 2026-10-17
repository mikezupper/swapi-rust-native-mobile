package dev.holocron.app

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performClick
import androidx.test.platform.app.InstrumentationRegistry
import dev.holocron.core.CharacterSession
import dev.holocron.core.CharacterState
import org.junit.Assert.assertEquals
import java.net.InetAddress
import java.net.ServerSocket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread
import org.junit.Rule
import org.junit.Test

class CharacterScreenTest {
    @get:Rule val compose = createComposeRule()

    @Test fun rustLoadingFailureRetryAndCharacterRenderThroughGeneratedBindings() {
        FixtureServer().use { server ->
            val releaseFailure = CountDownLatch(1)
            server.responses.add(Reply(503, "", releaseFailure))
            server.responses.add(Reply(200, fixture()))
            lateinit var controller: CharacterController
            compose.runOnUiThread { controller = CharacterController(CharacterSession.withEndpoint(server.endpoint)) }
            try {
                compose.setContent { HolocronTheme { CharacterRoute(controller) } }
                compose.runOnUiThread { controller.load() }
                compose.waitUntil(5_000) { controller.state.value.state is CharacterState.Loading }
                compose.onNodeWithText("Reading the stars").assertIsDisplayed()
                assertEquals("GET /api/people/1/ HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
                releaseFailure.countDown()
                compose.waitUntil(5_000) { controller.state.value.state is CharacterState.Failed }
                compose.onNodeWithText("Try again").performScrollTo().performClick()
                compose.waitUntil(5_000) { controller.state.value.state is CharacterState.Ready }
                compose.onNodeWithText("Luke Skywalker").performScrollTo().assertIsDisplayed()
                compose.onNodeWithText("172 cm").performScrollTo().assertIsDisplayed()
                compose.onNodeWithText("4 films").performScrollTo().assertIsDisplayed()
                assertEquals("GET /api/people/1/ HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            } finally { compose.runOnUiThread { controller.close() } }
        }
    }

    @Test fun disposingWhileResponseIsPendingLeavesNativeStateUnchanged() {
        FixtureServer().use { server ->
            val release = CountDownLatch(1)
            server.responses.add(Reply(200, fixture(), release))
            lateinit var controller: CharacterController
            compose.runOnUiThread {
                controller = CharacterController(CharacterSession.withEndpoint(server.endpoint))
                controller.load()
            }
            compose.waitUntil(5_000) { controller.state.value.state is CharacterState.Loading }
            assertEquals("GET /api/people/1/ HTTP/1.1", server.requests.poll(5, TimeUnit.SECONDS))
            val before = controller.state.value
            compose.runOnUiThread { controller.close(); controller.close() }
            release.countDown()
            server.completed.await(5, TimeUnit.SECONDS)
            compose.waitForIdle()
            assertEquals(before, controller.state.value)
        }
    }

    private fun fixture() = InstrumentationRegistry.getInstrumentation().context.assets
        .open("luke.json").bufferedReader().use { it.readText() }
}

internal data class Reply(val status: Int, val body: String, val release: CountDownLatch = CountDownLatch(0))

/** Test-only loopback HTTP server. All application networking still runs in Rust. */
internal class FixtureServer : AutoCloseable {
    private val socket = ServerSocket(0, 10, InetAddress.getByName("127.0.0.1"))
    val endpoint = "http://127.0.0.1:${socket.localPort}/api/"
    val responses = LinkedBlockingQueue<Reply>()
    val requests = LinkedBlockingQueue<String>()
    val completed = CountDownLatch(1)
    private val worker = thread(isDaemon = true, name = "fixture-http") {
        while (!socket.isClosed) {
            try {
                socket.accept().use { client ->
                    client.soTimeout = 5_000
                    val reader = client.getInputStream().bufferedReader()
                    val request = reader.readLine()
                    android.util.Log.d("FixtureServer", "${socket.localPort}: $request")
                    requests.add(request)
                    while (!reader.readLine().isNullOrEmpty()) { /* consume headers */ }
                    val reply = responses.poll(5, TimeUnit.SECONDS) ?: error("No fixture response for $request on ${socket.localPort}")
                    check(reply.release.await(5, TimeUnit.SECONDS))
                    val body = reply.body.toByteArray()
                    val headers = "HTTP/1.1 ${reply.status} Fixture\r\nContent-Type: application/json\r\nContent-Length: ${body.size}\r\nConnection: close\r\n\r\n"
                    client.getOutputStream().write(headers.toByteArray() + body)
                }
            } catch (error: java.io.IOException) {
                if (!socket.isClosed) { /* Cancellation may close the client socket. */ }
            } finally { completed.countDown() }
        }
    }
    override fun close() { socket.close(); worker.join(6_000) }
}
