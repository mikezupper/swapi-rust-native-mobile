package dev.holocron.app

import android.os.Handler
import android.os.Looper
import androidx.annotation.MainThread
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import dev.holocron.core.*
import java.lang.ref.WeakReference
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

class CatalogController(private val session: CatalogSession) : AutoCloseable {
    private val mutableState = MutableStateFlow(CatalogSnapshot(0uL, "", emptyList(), 0u, false, CatalogPhase.Idle, emptyCache()))
    val state = mutableState.asStateFlow()
    private var closed = false
    private val subscription = run {
        val owner = WeakReference(this)
        val main = Handler(Looper.getMainLooper())
        session.subscribe(object : CatalogListener {
            override fun onState(snapshot: CatalogSnapshot) {
                main.post {
                    owner.get()?.let { controller ->
                        if (!controller.closed && snapshot.revision > controller.mutableState.value.revision) {
                            controller.mutableState.value = snapshot
                        }
                    }
                }
            }
        })
    }
    @MainThread fun search(query: String) { if (!closed) session.search(query) }
    @MainThread fun refresh() { if (!closed) session.refresh() }
    @MainThread fun loadMore() { if (!closed) session.loadMore() }
    @MainThread fun retry() { if (!closed) session.retry() }
    @MainThread override fun close() {
        if (closed) return
        closed = true
        subscription.cancel(); subscription.destroy()
        session.shutdown(); session.destroy()
    }
}

class CatalogViewModel(private val saved: SavedStateHandle, category: CatalogCategory, recordId: UInt? = null, endpoint: String? = null, archive: ArchiveSession? = null, relatedIds: List<UInt>? = null) : ViewModel() {
    val query = saved.getStateFlow("query", "")
    val controller = CatalogController(if (relatedIds != null) archive?.relatedSession(category, relatedIds) ?: CatalogSession.relatedRecords(category, relatedIds, endpoint) else archive?.catalogSession(category, recordId) ?: endpoint?.let { CatalogSession.withEndpoint(it, category, recordId) } ?: CatalogSession(category, recordId)).also {
        if (query.value.isEmpty()) it.refresh() else it.search(query.value)
    }
    fun search(value: String) { saved["query"] = value; controller.search(value) }
    override fun onCleared() { controller.close() }
}
