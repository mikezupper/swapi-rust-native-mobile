package dev.holocron.app

import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.res.painterResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.holocron.core.*
import java.text.DateFormat
import java.util.Date

val LocalArchive = staticCompositionLocalOf<ArchiveController?> { null }
val LocalOpenArchive = staticCompositionLocalOf<() -> Unit> { {} }

@Composable fun ArchiveShortcut() {
    val archive = LocalArchive.current ?: return
    val snapshot by archive.state.collectAsStateWithLifecycle()
    FilledTonalButton(onClick = LocalOpenArchive.current, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) {
        Text(if (snapshot.offline) "Offline mode · Open saved archive" else "Saved archive · ${snapshot.favorites.size} favorites")
    }
}
@Composable fun CacheNotice(cache: CacheStatus) {
    if (cache.savedAt == null && !cache.savingFailed) return
    val date = cache.savedAt?.let { DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(it.toLong() * 1000)) }
    Text(when {
        cache.savingFailed -> "Live data · Could not save for offline use"
        cache.refreshFailed -> "Saved copy · Refresh unavailable · $date"
        cache.offline -> "Offline copy · Saved $date"
        cache.fromCache -> "Saved copy · Updated $date"
        else -> "Available offline · Updated $date"
    }, color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp)
}
@Composable fun FavoriteButton(category: CatalogCategory, id: UInt) {
    val archive = LocalArchive.current ?: return
    val snapshot by archive.state.collectAsStateWithLifecycle()
    val saved = snapshot.favorites.any { it.category == category && it.id == id }
    val haptic = LocalHapticFeedback.current
    val scale by animateFloatAsState(if (saved) 1.15f else 1f, label = "Favorite star")
    FilledTonalButton(onClick = {
        archive.favorite(category, id, !saved)
        haptic.performHapticFeedback(HapticFeedbackType.Confirm)
    }) {
        Icon(Icons.Filled.Star, null, Modifier.size(18.dp).graphicsLayer { scaleX = scale; scaleY = scale })
        Spacer(Modifier.width(8.dp))
        Text(if (saved) "Remove favorite" else "Add favorite")
    }
}

@Composable fun SavedArchiveScreen(archive: ArchiveController, onBack: () -> Unit, onRecord: (CatalogCategory, UInt) -> Unit) {
    val snapshot by archive.state.collectAsStateWithLifecycle()
    var query by remember { mutableStateOf(snapshot.query) }
    var downloads by remember { mutableStateOf(false) }
    var confirmClear by remember { mutableStateOf(false) }
    val running = snapshot.download as? DownloadPhase.Downloading
    if (confirmClear) AlertDialog(onDismissRequest = { confirmClear = false }, title = { Text("Clear saved cache?") },
        text = { Text("Favorites and their records stay on this device. Other saved records and downloaded collections will be removed.") },
        confirmButton = { TextButton(onClick = { archive.clearCache(); confirmClear = false }) { Text("Clear cache") } },
        dismissButton = { TextButton(onClick = { confirmClear = false }) { Text("Cancel") } })
    Surface(Modifier.fillMaxSize()) {
        LazyColumn(contentPadding = PaddingValues(bottom = 32.dp), verticalArrangement = Arrangement.spacedBy(16.dp), modifier = Modifier.imePadding()) {
            item {
                Box(Modifier.fillMaxWidth().clip(RoundedCornerShape(bottomStart = 28.dp, bottomEnd = 28.dp))) {
                    Image(painterResource(R.drawable.hero_starships), null, Modifier.matchParentSize(), contentScale = ContentScale.Crop)
                    Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(DesignTokens.Hero.background.copy(alpha = .5f), DesignTokens.Hero.background))))
                    Column(Modifier.statusBarsPadding().padding(24.dp)) {
                        IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to explore", tint = DesignTokens.Hero.accent) }
                        Spacer(Modifier.height(24.dp))
                        Text("YOUR PERSONAL HOLOCRON", fontSize = 11.sp, letterSpacing = 2.sp, color = DesignTokens.Hero.accent)
                        Text("The galaxy.\nTo go.", fontSize = 40.sp, lineHeight = 46.sp, color = DesignTokens.Hero.text, modifier = Modifier.semantics { heading() })
                        Text("Saved stories, wherever you travel.", color = DesignTokens.Hero.secondary, fontSize = 14.sp, modifier = Modifier.padding(top = 10.dp))
                    }
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text("Offline mode", fontSize = 18.sp)
                            Text("Browse saved records without a connection", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Switch(checked = snapshot.offline, onCheckedChange = archive::offline, modifier = Modifier.semantics { contentDescription = "Offline mode" })
                    }
                    OutlinedButton(onClick = { downloads = !downloads }, modifier = Modifier.fillMaxWidth()) { Text(if (downloads) "Hide collection downloads" else "Download collections") }
                    if (downloads) Column(Modifier.animateContentSize(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        snapshot.collections.forEach { collection ->
                            Surface(color = MaterialTheme.colorScheme.surfaceContainer, shape = RoundedCornerShape(20.dp)) {
                                Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                    Text(collection.category.title, fontSize = 18.sp)
                                    Text(if (collection.complete) "Complete · ${collection.savedCount} records" else "${collection.savedCount} saved · ${collection.totalCount?.toString() ?: "?"} total · Partial", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    if (running?.category == collection.category) {
                                        LinearProgressIndicator(Modifier.fillMaxWidth())
                                        Text("Downloading ${running.completed} of ${running.total} records", fontSize = 12.sp)
                                        TextButton(onClick = archive::cancelDownload) { Text("Cancel download") }
                                    } else {
                                        TextButton(enabled = !snapshot.offline && running == null, onClick = { archive.download(collection.category) }) { Text(if (collection.complete) "Update ${collection.category.title.lowercase()}" else "Download ${collection.category.title.lowercase()}") }
                                    }
                                }
                            }
                        }
                        TextButton(onClick = { confirmClear = true }) { Text("Clear saved cache") }
                    }
                    when (val download = snapshot.download) {
                        is DownloadPhase.Ready -> Text("${download.category.title}: ${download.count} records ready offline", color = MaterialTheme.colorScheme.primary)
                        is DownloadPhase.Failed -> Column {
                            Text("Download interrupted. Saved records are still available.")
                            Text(failureMessage(download.kind), fontSize = 12.sp)
                            TextButton(enabled = !snapshot.offline, onClick = { archive.download(download.category) }) { Text("Retry download") }
                        }
                        else -> {}
                    }
                    snapshot.error?.let { Text(failureMessage(it), color = MaterialTheme.colorScheme.error) }
                    OutlinedTextField(query, { query = it; archive.search(it) }, label = { Text("Search saved records") }, singleLine = true, modifier = Modifier.fillMaxWidth(), shape = RoundedCornerShape(18.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        FilterChip(!snapshot.favoritesOnly, { archive.favoritesOnly(false) }, { Text("All saved") })
                        FilterChip(snapshot.favoritesOnly, { archive.favoritesOnly(true) }, { Text("Favorites") })
                    }
                    Text("${snapshot.entries.size} saved records", fontSize = 22.sp, modifier = Modifier.semantics { heading() })
                    Text("Search covers saved records only. Download a collection to keep every story.", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            if (snapshot.entries.isEmpty()) item { Text("No saved signals yet. Explore a collection or download it for your next journey.", modifier = Modifier.padding(horizontal = 24.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
            items(snapshot.entries, key = { "${it.category}-${it.entry.id}" }) { saved ->
                Surface(onClick = { onRecord(saved.category, saved.entry.id) }, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp).animateItem(), shape = RoundedCornerShape(24.dp), color = MaterialTheme.colorScheme.surfaceContainer) {
                    Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                        Image(painterResource(saved.category.artwork), null, Modifier.size(64.dp).clip(RoundedCornerShape(16.dp)), contentScale = ContentScale.Crop)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(saved.entry.name, fontSize = 18.sp)
                            Text(saved.category.title + if (saved.favorite) " · Favorite" else " · Saved", fontSize = 12.sp, color = MaterialTheme.colorScheme.primary)
                        }
                    }
                }
            }
            item { Spacer(Modifier.navigationBarsPadding()) }
        }
    }
}
