package dev.holocron.app

import android.os.Handler
import android.os.Looper
import androidx.lifecycle.ViewModel
import dev.holocron.core.*
import java.lang.ref.WeakReference
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

fun emptyExploration() = ExplorationSnapshot(null, emptyList(), emptyList(), emptyList(), emptyList(), emptyList())

fun emptyCache() = CacheStatus(null, false, false, false, false)

/** The app-scoped Rust archive owns persistence and all request/cache policy. */
class ArchiveController(val session: ArchiveSession, private val publish: (ArchiveSnapshot) -> Unit = {}) : AutoCloseable {
    private val mutableState = MutableStateFlow(ArchiveSnapshot(0uL, "", false, false, emptyList(), emptyList(), emptyList(), DownloadPhase.Idle, null, emptyExploration()))
    val state = mutableState.asStateFlow()
    private var closed = false
    private val subscription = run {
        val owner = WeakReference(this)
        val main = Handler(Looper.getMainLooper())
        session.subscribe(object : ArchiveListener {
            override fun onState(snapshot: ArchiveSnapshot) {
                main.post { owner.get()?.let { controller ->
                    if (!controller.closed && snapshot.revision > controller.mutableState.value.revision) { controller.mutableState.value = snapshot; controller.publish(snapshot) }
                } }
            }
        })
    }
    fun favorite(category: CatalogCategory, id: UInt, value: Boolean) { if (!closed) session.setFavorite(category, id, value) }
    fun search(value: String) { if (!closed) session.search(value) }
    fun favoritesOnly(value: Boolean) { if (!closed) session.setFavoritesOnly(value) }
    fun offline(value: Boolean) { if (!closed) session.setOffline(value) }
    fun download(category: CatalogCategory) { if (!closed) session.download(category) }
    fun cancelDownload() { if (!closed) session.cancelDownload() }
    fun clearCache() { if (!closed) session.clearCache() }
    override fun close() {
        if (closed) return
        closed = true
        subscription.cancel(); subscription.destroy(); session.shutdown(); session.destroy()
    }
}
class ArchiveViewModel(path: String, endpoint: String? = null, context: android.content.Context? = null) : ViewModel() {
    val controller = runCatching { ArchiveController(endpoint?.let { ArchiveSession.withEndpoint(path, it) } ?: ArchiveSession(path)) { snapshot -> if (endpoint == null && context != null) HolocronWidget.publish(context, snapshot.exploration.widget) } }.getOrNull()
    override fun onCleared() { controller?.close() }
}
