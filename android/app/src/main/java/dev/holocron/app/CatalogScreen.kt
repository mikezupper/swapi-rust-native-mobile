package dev.holocron.app

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.holocron.core.*

val CatalogCategory.title: String get() = when (this) {
    CatalogCategory.PEOPLE -> "People"; CatalogCategory.PLANETS -> "Planets"
    CatalogCategory.STARSHIPS -> "Starships"; CatalogCategory.VEHICLES -> "Vehicles"
    CatalogCategory.SPECIES -> "Species"; CatalogCategory.FILMS -> "Films"
}
val CatalogCategory.headline: String get() = when (this) {
    CatalogCategory.PEOPLE -> "A galaxy of\nstories."
    CatalogCategory.PLANETS -> "Worlds beyond\nthe horizon."
    CatalogCategory.STARSHIPS -> "Built for\nthe stars."
    CatalogCategory.VEHICLES -> "Adventure\nin motion."
    CatalogCategory.SPECIES -> "Life across\nthe galaxy."
    CatalogCategory.FILMS -> "The stories\nthat started it."
}
val CatalogCategory.subtitle: String get() = when (this) {
    CatalogCategory.PEOPLE -> "Discover the people behind the legends."
    CatalogCategory.PLANETS -> "Desert suns. Ocean worlds. Distant homes."
    CatalogCategory.STARSHIPS -> "Explore the vessels that cross the galaxy."
    CatalogCategory.VEHICLES -> "From desert crawlers to soaring speeders."
    CatalogCategory.SPECIES -> "Discover the galaxy’s many forms of life."
    CatalogCategory.FILMS -> "Explore the saga, one episode at a time."
}

@Composable
fun CategoryPicker(selected: CatalogCategory, onSelect: (CatalogCategory) -> Unit) {
    val scale = LocalDensity.current.fontScale
    val columns = if (scale > 1.8f) 1 else if (scale > 1.3f) 2 else 3
    Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        CatalogCategory.entries.chunked(columns).forEach { categories ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                categories.forEach { category ->
                    FilterChip(selected = selected == category, onClick = { if (selected != category) onSelect(category) },
                        label = { Text(category.title, fontSize = 13.sp) },
                        colors = FilterChipDefaults.filterChipColors(containerColor = MaterialTheme.colorScheme.surfaceContainer,
                            labelColor = MaterialTheme.colorScheme.onSurfaceVariant, selectedContainerColor = MaterialTheme.colorScheme.primary,
                            selectedLabelColor = MaterialTheme.colorScheme.onPrimary),
                        modifier = Modifier.weight(1f).heightIn(min = 48.dp), shape = RoundedCornerShape(14.dp))
                }
            }
        }
    }
}

@Composable
fun CatalogScreen(category: CatalogCategory, snapshot: CatalogSnapshot, query: String, onSearch: (String) -> Unit,
                  onRefresh: () -> Unit, onMore: () -> Unit, onRetry: () -> Unit,
                  onCategory: (CatalogCategory) -> Unit, onRecord: (UInt) -> Unit) {
    val failure = snapshot.phase as? CatalogPhase.Failed
    ArchiveScreen(category, snapshot.items.map { ArchiveItem(it.id, it.name, it.facts.take(2).joinToString(" · ") { fact -> "${fact.label}: ${fact.value}" }) },
        snapshot.total, snapshot.hasMore, snapshot.phase == CatalogPhase.Ready,
        snapshot.phase == CatalogPhase.Loading || snapshot.phase == CatalogPhase.Refreshing || snapshot.phase == CatalogPhase.LoadingMore,
        failure?.kind, failure?.retryable == true, query, onSearch, onRefresh, onMore, onRetry, onCategory, onRecord)
}

@Composable
fun CatalogDetailScreen(category: CatalogCategory, snapshot: CatalogSnapshot, onRefresh: () -> Unit, onRetry: () -> Unit, onBack: () -> Unit) {
    val record = snapshot.items.firstOrNull()
    val busy = snapshot.phase == CatalogPhase.Loading || snapshot.phase == CatalogPhase.Refreshing
    Surface(Modifier.fillMaxSize()) {
        Column(Modifier.navigationBarsPadding()) {
            Box(Modifier.fillMaxWidth().windowInsetsTopHeight(WindowInsets.statusBars).background(DesignTokens.Hero.background))
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(horizontal = 12.dp)) {
                IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to ${category.title.lowercase()}") }
                Text(category.title, color = MaterialTheme.colorScheme.primary, fontSize = 14.sp)
            }
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                Box(Modifier.fillMaxWidth()) {
                    Image(painterResource(R.drawable.desert_orbit), null, Modifier.matchParentSize(), contentScale = ContentScale.Crop)
                    Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(DesignTokens.Hero.background.copy(alpha = .5f), DesignTokens.Hero.background))))
                    Column(Modifier.padding(24.dp)) {
                        Spacer(Modifier.height(64.dp))
                        Text("${category.title.uppercase()} / ${record?.id?.toString()?.padStart(3, '0') ?: "…"}", color = DesignTokens.Hero.accent, fontSize = 12.sp, letterSpacing = 2.sp)
                        Text(record?.name ?: "Reading the stars", fontSize = 38.sp, lineHeight = 44.sp,
                            color = DesignTokens.Hero.text, fontWeight = FontWeight.Medium, modifier = Modifier.padding(top = 12.dp).semantics { heading() })
                    }
                }
                Column(Modifier.fillMaxWidth().padding(24.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
                    if (busy) LinearProgressIndicator(Modifier.fillMaxWidth())
                    val failure = snapshot.phase as? CatalogPhase.Failed
                    if (failure != null) {
                        Text("Signal interrupted", fontSize = 22.sp)
                        Text(failureMessage(failure.kind))
                        if (failure.retryable) FilledTonalButton(onClick = onRetry) { Text("Try again") }
                    }
                    if (record != null) {
                        Text("The details", fontSize = 24.sp, modifier = Modifier.semantics { heading() })
                        Surface(modifier = Modifier.fillMaxWidth(), color = MaterialTheme.colorScheme.surfaceContainer, shape = RoundedCornerShape(24.dp)) {
                            Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
                                record.facts.forEach { fact ->
                                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                        Text(fact.label, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                        Text(fact.value, fontSize = 18.sp)
                                    }
                                }
                            }
                        }
                        record.description?.let { crawl ->
                            Text("Opening crawl", fontSize = 24.sp, modifier = Modifier.semantics { heading() })
                            Text(crawl, fontSize = 17.sp, lineHeight = 28.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Text("Record from the Star Wars API.", color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp)
                    }
                    OutlinedButton(onClick = onRefresh, enabled = !busy) { Text("Refresh record") }
                }
            }
        }
    }
}
