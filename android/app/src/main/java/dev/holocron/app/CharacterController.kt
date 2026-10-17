package dev.holocron.app

import android.os.Handler
import android.os.Looper
import androidx.annotation.MainThread
import androidx.lifecycle.ViewModel
import dev.holocron.core.CharacterListener
import dev.holocron.core.CharacterSession
import dev.holocron.core.CharacterSnapshot
import dev.holocron.core.CharacterState
import java.lang.ref.WeakReference
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Adapts Rust snapshots to the Android main thread; owns no request policy. */
class CharacterController(private val session: CharacterSession = CharacterSession(), private val personId: UInt = 1u) : AutoCloseable {
    private val mutableState = MutableStateFlow(CharacterSnapshot(0uL, CharacterState.Idle))
    val state = mutableState.asStateFlow()
    private var closed = false // Accessed on the main thread only.
    private val subscription = run {
        val owner = WeakReference(this)
        val main = Handler(Looper.getMainLooper())
        session.subscribe(object : CharacterListener {
            override fun onState(snapshot: CharacterSnapshot) {
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

    @MainThread fun load(id: UInt = personId) { if (!closed) session.load(id) }
    @MainThread fun retry() { if (!closed) session.retry() }
    @MainThread fun cancel() { if (!closed) session.cancel() }
    @MainThread override fun close() {
        if (closed) return
        // Invalidate queued deliveries before asking Rust to stop callbacks.
        closed = true
        subscription.cancel()
        subscription.destroy()
        session.shutdown()
        session.destroy()
    }
}

class CharacterViewModel(id: UInt = 1u, endpoint: String? = null) : ViewModel() {
    val controller = CharacterController(endpoint?.let { CharacterSession.withEndpoint(it) } ?: CharacterSession(), id).also { it.load() }
    override fun onCleared() { controller.close() }
}
