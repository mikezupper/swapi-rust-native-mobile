package dev.holocron.app

import android.graphics.Bitmap
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File

internal fun captureScreen(name: String) {
    val instrumentation = InstrumentationRegistry.getInstrumentation()
    val directory = File(instrumentation.targetContext.filesDir, "screenshots").also { it.mkdirs() }
    val bitmap = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
    File(directory, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    bitmap.recycle()
}
