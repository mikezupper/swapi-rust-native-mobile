package dev.holocron.app

import android.graphics.Bitmap
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.graphics.asAndroidBitmap

internal fun captureScreen(name: String) {
    val instrumentation = InstrumentationRegistry.getInstrumentation()
    val directory = File(instrumentation.targetContext.filesDir, "screenshots").also { it.mkdirs() }
    val bitmap = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
    File(directory, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    bitmap.recycle()
}

internal fun ComposeContentTestRule.captureContent(name: String) {
    waitForIdle()
    val bitmap = onRoot().captureToImage().asAndroidBitmap()
    val directory = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "screenshots").also { it.mkdirs() }
    File(directory, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
}
