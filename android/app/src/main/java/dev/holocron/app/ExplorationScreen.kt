package dev.holocron.app

import android.animation.ValueAnimator
import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.holocron.core.*

val LocalOpenRecord = staticCompositionLocalOf<(CatalogCategory, UInt) -> Unit> { { _, _ -> } }
val LocalOpenRelated = staticCompositionLocalOf<(CatalogRelation) -> Unit> { {} }
val LocalOpenDiscover = staticCompositionLocalOf<() -> Unit> { {} }
val LocalSharedScope = staticCompositionLocalOf<SharedTransitionScope?> { null }
val LocalNavigationAnimation = staticCompositionLocalOf<AnimatedVisibilityScope?> { null }

@Composable
fun Modifier.recordBounds(category: CatalogCategory, id: UInt): Modifier {
    val shared = LocalSharedScope.current
    val visibility = LocalNavigationAnimation.current
    return if (shared != null && visibility != null && ValueAnimator.areAnimatorsEnabled()) with(shared) {
        this@recordBounds.sharedBounds(rememberSharedContentState("${category.name}/$id"), visibility,
            boundsTransform = { _, _ -> tween(400, easing = FastOutSlowInEasing) },
            resizeMode = SharedTransitionScope.ResizeMode.scaleToBounds())
    } else this
}

@Composable fun RelatedRecords(groups: List<CatalogRelation>) {
    if (groups.isEmpty()) return
    val open = LocalOpenRelated.current
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Connected stories", fontSize = 24.sp, modifier = Modifier.semantics { heading() })
        Text("Follow a connection into the galaxy.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        groups.forEach { group ->
            OutlinedButton(onClick = { open(group) }, modifier = Modifier.fillMaxWidth()) {
                Text("${group.label} · ${group.ids.size}", Modifier.weight(1f))
                Text("→")
            }
        }
    }
}

@Composable fun CompareShipButton(id: UInt) {
    val archive = LocalArchive.current ?: return
    val state by archive.state.collectAsStateWithLifecycle()
    val ships = state.exploration.ships
    val selected = ships.any { it.id == id }
    val haptic = LocalHapticFeedback.current
    OutlinedButton(onClick = { archive.session.selectShip(id, !selected); haptic.performHapticFeedback(HapticFeedbackType.LongPress) },
        enabled = selected || ships.size < 2, modifier = Modifier.semantics { contentDescription = if (selected) "Remove ship $id from comparison" else "Compare ship $id" }) {
        Text(if (selected) "Remove from comparison" else if (ships.size == 2) "Comparison full · 2 ships" else "＋ Compare ship")
    }
}

@Composable fun ComparisonTray(archive: ArchiveController, open: () -> Unit) {
    val snapshot by archive.state.collectAsStateWithLifecycle()
    val ships = snapshot.exploration.ships
    AnimatedVisibility(ships.isNotEmpty(), enter = slideInVertically { it } + fadeIn(), exit = slideOutVertically { it } + fadeOut()) {
        Surface(color = MaterialTheme.colorScheme.surfaceContainerHigh, tonalElevation = 8.dp, shadowElevation = 10.dp) {
            Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 20.dp, vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("SHIP COMPARISON · ${ships.size}/2", fontSize = 11.sp, letterSpacing = 1.5.sp, color = MaterialTheme.colorScheme.primary)
                ships.forEach { ship ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(ship.name, Modifier.weight(1f), fontSize = 14.sp)
                        TextButton(onClick = { archive.session.selectShip(ship.id, false) }) { Text("Remove", Modifier.semantics { contentDescription = "Remove ${ship.name}" }) }
                    }
                }
                Button(onClick = open, enabled = ships.size == 2, modifier = Modifier.fillMaxWidth()) { Text(if (ships.size == 2) "Compare starships" else "Choose one more starship") }
            }
        }
    }
}

@Composable fun ExplorationScaffold(title: String, onBack: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    Surface(Modifier.fillMaxSize()) {
        Column(Modifier.statusBarsPadding().navigationBarsPadding()) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
                Text(title, fontSize = 18.sp, color = MaterialTheme.colorScheme.primary)
            }
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp), verticalArrangement = Arrangement.spacedBy(20.dp), content = content)
        }
    }
}

@Composable fun DiscoveryScreen(archive: ArchiveController, onBack: () -> Unit) {
    val snapshot by archive.state.collectAsStateWithLifecycle()
    val discovery = snapshot.exploration
    val open = LocalOpenRecord.current
    LaunchedEffect(archive) { archive.session.discover() }
    ExplorationScaffold("Discover", onBack) {
        Text("Your next\ngalactic detour.", fontSize = 38.sp, lineHeight = 44.sp, modifier = Modifier.semantics { heading() })
        Text(if (snapshot.offline) "Discoveries from your saved galaxy." else "A familiar world. An unexpected connection.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        discovery.featured?.let { saved ->
            Card(onClick = { open(saved.category, saved.entry.id) }, modifier = Modifier.fillMaxWidth().recordBounds(saved.category, saved.entry.id), shape = RoundedCornerShape(28.dp)) {
                Box(Modifier.fillMaxWidth()) {
                    Image(painterResource(saved.category.artwork), null, Modifier.matchParentSize(), contentScale = ContentScale.Crop)
                    Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(Color.Transparent, DesignTokens.Hero.background))))
                    Column(Modifier.padding(24.dp)) {
                        Text("FEATURED WORLD", fontSize = 12.sp, color = DesignTokens.Hero.accent, letterSpacing = 2.sp, modifier = Modifier.background(DesignTokens.Hero.background.copy(alpha = .92f), RoundedCornerShape(8.dp)).padding(horizontal = 8.dp, vertical = 6.dp))
                        Spacer(Modifier.height(100.dp))
                        Text(saved.entry.name, fontSize = 30.sp, color = DesignTokens.Hero.text)
                        Text(saved.entry.facts.take(2).joinToString(" · ") { it.value }, color = DesignTokens.Hero.secondary)
                        Text("Explore world →", color = DesignTokens.Hero.accent, modifier = Modifier.padding(top = 16.dp))
                    }
                }
            }
        } ?: Column {
            Text(if (snapshot.offline) "Save a planet to feature it here." else "Preparing your first discovery…")
            TextButton(onClick = { archive.session.discover() }) { Text("Find a world") }
        }
        if (snapshot.error != null) Text("Discovery could not update. Saved records remain available.", color = MaterialTheme.colorScheme.error)
        if (discovery.recent.isNotEmpty()) {
            Text("Recently explored", fontSize = 24.sp, modifier = Modifier.semantics { heading() })
            discovery.recent.forEach { DiscoveryRow(it, open) }
        }
        Text("Continue exploring", fontSize = 24.sp, modifier = Modifier.semantics { heading() })
        if (discovery.suggestions.isEmpty()) Text("Open a few records to build your own trail through the stars.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        discovery.suggestions.forEach { DiscoveryRow(it, open) }
        Text("Add Holocron to your home screen for a favorite and a saved discovery.", color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 13.sp)
        val context = androidx.compose.ui.platform.LocalContext.current
        OutlinedButton(onClick = { HolocronWidget.pin(context) }) { Text("Add home-screen widget") }
    }
}

@Composable private fun DiscoveryRow(saved: SavedEntry, open: (CatalogCategory, UInt) -> Unit) {
    Card(onClick = { open(saved.category, saved.entry.id) }, modifier = Modifier.fillMaxWidth(), colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer)) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Image(painterResource(saved.category.artwork), null, Modifier.size(56.dp).clip(RoundedCornerShape(14.dp)), contentScale = ContentScale.Crop)
            Column(Modifier.weight(1f)) { Text(saved.entry.name, fontWeight = FontWeight.Medium); Text(saved.category.title, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant) }
            Text("→", color = MaterialTheme.colorScheme.primary)
        }
    }
}

@Composable fun ComparisonScreen(archive: ArchiveController, onBack: () -> Unit) {
    val snapshot by archive.state.collectAsStateWithLifecycle()
    val comparison = snapshot.exploration
    var revealed by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { revealed = true }
    ExplorationScaffold("Starship comparison", onBack) {
        Text("Choose your\nride.", fontSize = 38.sp, lineHeight = 44.sp, modifier = Modifier.semantics { heading() })
        if (comparison.ships.size != 2) Text("Select two starships from their collection or detail screens.")
        comparison.ships.forEachIndexed { index, ship ->
            Surface(color = if (index == 0) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.tertiaryContainer, shape = RoundedCornerShape(20.dp)) {
                Column(Modifier.fillMaxWidth().padding(20.dp)) {
                    Text(if (index == 0) "SHIP A" else "SHIP B", letterSpacing = 2.sp, fontSize = 12.sp)
                    Text(ship.name, fontSize = 24.sp)
                    Text(ship.facts.firstOrNull { it.label == "Model" }?.value ?: "", fontSize = 13.sp)
                }
            }
        }
        comparison.metrics.forEach { metric ->
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(metric.label, fontSize = 22.sp, modifier = Modifier.semantics { heading() })
                metric.values.forEachIndexed { index, value ->
                    val color = if (index == 0) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.tertiary
                    Text("${comparison.ships[index].name}: ${value.display}", fontSize = 14.sp)
                    value.fraction?.let { fraction ->
                        val animated by animateFloatAsState(if (revealed) fraction.toFloat() else 0f, tween(600), label = "Comparison bar")
                        Box(Modifier.fillMaxWidth().height(8.dp).clip(RoundedCornerShape(4.dp)).background(color.copy(alpha = .15f)).clearAndSetSemantics {}) {
                            Box(Modifier.fillMaxWidth(animated).fillMaxHeight().background(color))
                        }
                    }
                }
                if (metric.lowerIsBetter) Text("Lower hyperdrive ratings are faster.", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Text("Bars show relative values, not an overall score. Unknown values and crew ranges keep their original meaning.", fontSize = 13.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable fun FilmExperience(title: String, crawl: String) {
    var cinematic by rememberSaveable(title) { mutableStateOf(false) }
    var playing by rememberSaveable(title) { mutableStateOf(false) }
    var elapsed by rememberSaveable(title) { mutableFloatStateOf(0f) }
    val lifecycle by LocalLifecycleOwner.current.lifecycle.currentStateFlow.collectAsStateWithLifecycle()
    val reduced = !ValueAnimator.areAnimatorsEnabled()
    val large = LocalDensity.current.fontScale > 1.3f
    val duration = (crawl.length / 12f).coerceIn(35f, 160f)
    LaunchedEffect(lifecycle, reduced, large) { if (!lifecycle.isAtLeast(Lifecycle.State.RESUMED) || reduced || large) playing = false }
    LaunchedEffect(playing, cinematic, reduced, large, lifecycle) {
        if (playing && cinematic && !reduced && !large && lifecycle.isAtLeast(Lifecycle.State.RESUMED)) {
            var previous = withFrameNanos { it }
            while (elapsed < duration) {
                val now = withFrameNanos { it }; elapsed = (elapsed + (now - previous).coerceAtMost(100_000_000) / 1_000_000_000f).coerceAtMost(duration); previous = now
            }
            playing = false
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Opening crawl", fontSize = 24.sp, modifier = Modifier.semantics { heading() })
        TextButton(onClick = { cinematic = !cinematic; playing = false }) { Text(if (cinematic) "Read opening crawl" else "Cinematic opening crawl") }
        if (cinematic && !reduced && !large) {
            var textHeight by remember { mutableIntStateOf(0) }
            val density = LocalDensity.current
            Box(Modifier.fillMaxWidth().height(360.dp).clip(RoundedCornerShape(24.dp)).background(DesignTokens.Hero.background).semantics { contentDescription = "Animated opening crawl for $title. Use Read opening crawl for the complete text." }) {
                Image(painterResource(CatalogCategory.FILMS.artwork), null, Modifier.matchParentSize().graphicsLayer { alpha = .18f }, contentScale = ContentScale.Crop)
                Text("$title\n\n$crawl", color = DesignTokens.Dark.warm, fontSize = 23.sp, lineHeight = 34.sp, fontWeight = FontWeight.SemiBold, textAlign = TextAlign.Center,
                    onTextLayout = { textHeight = it.size.height }, modifier = Modifier.fillMaxWidth().padding(horizontal = 30.dp).wrapContentHeight(unbounded = true, align = Alignment.Top).graphicsLayer {
                        val viewport = with(density) { 360.dp.toPx() }
                        translationY = viewport - elapsed / duration * (viewport + textHeight)
                        rotationX = 22f; cameraDistance = 24 * density.density
                    }.clearAndSetSemantics {})
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { if (elapsed >= duration) elapsed = 0f; playing = !playing }) { Text(if (playing) "Pause crawl" else if (elapsed >= duration) "Replay crawl" else "Play crawl") }
                OutlinedButton(onClick = { elapsed = 0f; playing = false }) { Text("Restart crawl") }
            }
            Slider(value = elapsed / duration, onValueChange = { elapsed = it * duration; playing = false }, modifier = Modifier.semantics { contentDescription = "Crawl progress" })
        } else {
            if (cinematic) Text("Reading mode follows your text and motion preferences.", fontSize = 13.sp)
            Text(crawl, fontSize = 17.sp, lineHeight = 28.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
