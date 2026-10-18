package dev.holocron.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.holocron.core.Character
import dev.holocron.core.CatalogCategory
import dev.holocron.core.CharacterState
import dev.holocron.core.FailureKind
import dev.holocron.core.Measurement
import java.text.NumberFormat

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT))
        setContent {
            val endpoint = if (BuildConfig.DEBUG) intent.getStringExtra("endpoint") else null
            val archive: ArchiveViewModel = viewModel(factory = viewModelFactory { initializer { ArchiveViewModel(java.io.File(filesDir, "archive.sqlite").path, endpoint, applicationContext) } })
            HolocronTheme { HolocronNavigation(endpoint = endpoint, archive = archive.controller, storageUnavailable = archive.controller == null, initialLink = intent.dataString) }
        }
    }
}

@Composable
fun HolocronTheme(content: @Composable () -> Unit) {
    val colors = if (isSystemInDarkTheme()) with(DesignTokens.Dark) {
        darkColorScheme(primary = accent, onPrimary = onAccent, background = background,
            onBackground = text, surface = background, onSurface = text,
            primaryContainer = accent.copy(alpha = .12f), onPrimaryContainer = accent,
            secondary = accent, onSecondary = onAccent, secondaryContainer = accent.copy(alpha = .12f), onSecondaryContainer = accent,
            tertiary = warm, onTertiary = onAccent, tertiaryContainer = warm.copy(alpha = .12f), onTertiaryContainer = warm,
            surfaceContainer = surface, surfaceContainerHigh = surface, surfaceContainerHighest = surface,
            surfaceContainerLow = surface, surfaceContainerLowest = background, surfaceVariant = androidx.compose.ui.graphics.lerp(surface, background, .1f),
            onSurfaceVariant = secondary, outline = secondary, outlineVariant = outline)
    } else with(DesignTokens.Light) {
        lightColorScheme(primary = accent, onPrimary = onAccent, background = background,
            onBackground = text, surface = background, onSurface = text,
            primaryContainer = accent.copy(alpha = .12f), onPrimaryContainer = accent,
            secondary = accent, onSecondary = onAccent, secondaryContainer = accent.copy(alpha = .12f), onSecondaryContainer = accent,
            tertiary = warm, onTertiary = onAccent, tertiaryContainer = warm.copy(alpha = .12f), onTertiaryContainer = warm,
            surfaceContainer = surface, surfaceContainerHigh = surface, surfaceContainerHighest = surface,
            surfaceContainerLow = surface, surfaceContainerLowest = background, surfaceVariant = androidx.compose.ui.graphics.lerp(surface, background, .1f),
            onSurfaceVariant = secondary, outline = secondary, outlineVariant = outline)
    }
    MaterialTheme(colorScheme = colors, content = content)
}

@Composable
fun CharacterRoute(controller: CharacterController) {
    val snapshot by controller.state.collectAsStateWithLifecycle()
    CharacterScreen(snapshot.state, onRetry = controller::retry, onLoad = { controller.load() }, cache = snapshot.cache)
}

@Composable
fun CharacterScreen(state: CharacterState, onRetry: () -> Unit, onLoad: () -> Unit, cache: dev.holocron.core.CacheStatus = emptyCache()) {
    val character = (state as? CharacterState.Ready)?.character
    val archive = LocalArchive.current
    LaunchedEffect(character?.id) { character?.let { archive?.session?.recordVisit(CatalogCategory.PEOPLE, it.id) } }
    Surface(Modifier.fillMaxSize()) {
        Box(Modifier.fillMaxSize()) {
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).navigationBarsPadding()) {
                Box(Modifier.fillMaxWidth().then(if (character != null) Modifier.recordBounds(CatalogCategory.PEOPLE, character.id) else Modifier).heightIn(min = 390.dp).clip(RoundedCornerShape(bottomStart = DesignTokens.Shape.hero.dp, bottomEnd = DesignTokens.Shape.hero.dp))) {
                    Image(painterResource(CatalogCategory.PEOPLE.artwork), null, Modifier.matchParentSize(), contentScale = ContentScale.Crop)
                    Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(DesignTokens.Hero.background.copy(alpha = 0.4f), Color.Transparent, DesignTokens.Hero.background))))
                    Column(Modifier.fillMaxWidth().statusBarsPadding().padding(DesignTokens.Space.lg.dp)) {
                        Text("HOLOCRON  ✦", color = DesignTokens.Hero.text, fontSize = 13.sp, letterSpacing = 2.sp, fontWeight = FontWeight.SemiBold)
                        Spacer(Modifier.height(166.dp))
                        Text("THE GALACTIC ARCHIVE" + (character?.let { " / ${it.id.toString().padStart(3, '0')}" } ?: ""), color = DesignTokens.Hero.accent, fontSize = 10.sp, letterSpacing = 1.6.sp, modifier = Modifier.background(DesignTokens.Hero.background.copy(alpha = .85f), RoundedCornerShape(8.dp)).padding(8.dp))
                        Spacer(Modifier.height(12.dp))
                        Text(character?.name ?: if (state is CharacterState.Failed) "Signal interrupted" else "Reading the stars", color = DesignTokens.Hero.text, fontSize = DesignTokens.Typography.hero.sp, lineHeight = 46.sp, fontWeight = FontWeight.Medium, letterSpacing = (-1.5).sp, modifier = Modifier.widthIn(max = 290.dp).semantics { heading() })
                        Spacer(Modifier.height(12.dp))
                        Text("One life. A galaxy of connections.", color = DesignTokens.Hero.secondary, fontSize = 13.sp)
                    }
                }
                Column(Modifier.padding(DesignTokens.Space.lg.dp), verticalArrangement = Arrangement.spacedBy(DesignTokens.Shape.card.dp)) {
                    when (state) {
                        is CharacterState.Ready -> {
                            CacheNotice(cache)
                            FavoriteButton(CatalogCategory.PEOPLE, state.character.id)
                            RelatedRecords(state.character.related)
                            CharacterDetails(state.character, onLoad)
                        }
                        is CharacterState.Failed -> {
                            Text(failureMessage(state.kind), fontSize = 18.sp, lineHeight = 26.sp)
                            if (state.retryable) FilledTonalButton(onClick = onRetry) { Text("Try again") }
                        }
                        is CharacterState.Loading -> {
                            LinearProgressIndicator(Modifier.fillMaxWidth(), color = MaterialTheme.colorScheme.primary)
                            Text("Connecting to the galactic archive…", color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        CharacterState.Idle -> FilledTonalButton(onClick = onLoad) { Text("Open the archive") }
                    }
                }
            }
            Box(Modifier.fillMaxWidth().windowInsetsTopHeight(WindowInsets.statusBars).background(DesignTokens.Hero.background.copy(alpha = 0.67f)))
        }
    }
}

@Composable
private fun CharacterDetails(character: Character, onLoad: () -> Unit) {
    val facts: @Composable () -> Unit = {
        Fact("Height", measurement(character.heightCm, "cm"))
        Fact("Mass", measurement(character.massKg, "kg"))
        Fact("Born", character.birthYear)
    }
    if (LocalDensity.current.fontScale > 1.3f) {
        Column(Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainer, RoundedCornerShape(DesignTokens.Shape.card.dp)).padding(16.dp), verticalArrangement = Arrangement.spacedBy(20.dp)) { facts() }
    } else {
        Row(Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainer, RoundedCornerShape(DesignTokens.Shape.card.dp)).border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(DesignTokens.Shape.card.dp)).padding(vertical = 22.dp)) {
            Fact("Height", measurement(character.heightCm, "cm"), Modifier.weight(1f))
            Fact("Mass", measurement(character.massKg, "kg"), Modifier.weight(1f))
            Fact("Born", character.birthYear, Modifier.weight(1f))
        }
    }
    Column {
        Text("The details", fontSize = DesignTokens.Typography.section.sp, fontWeight = FontWeight.Medium, modifier = Modifier.semantics { heading() })
        Spacer(Modifier.height(12.dp))
        DetailRow("Eye color", character.eyeColor)
        DetailRow("Hair color", character.hairColor)
        DetailRow("Film appearances", "${character.filmCount} films")
    }
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Icon(Icons.Outlined.Info, null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
        Text("Record from the Star Wars API.\nBBY: Before the Battle of Yavin.", fontSize = 12.sp, lineHeight = 18.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    OutlinedButton(onClick = onLoad, modifier = Modifier.fillMaxWidth()) { Text("Refresh record") }
}

@Composable
private fun Fact(label: String, value: String, modifier: Modifier = Modifier) {
    Column(modifier.padding(horizontal = 14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 11.sp)
        Text(value, fontSize = 18.sp, fontWeight = FontWeight.Medium)
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 16.dp), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 14.sp, modifier = Modifier.weight(1f))
        Spacer(Modifier.width(16.dp))
        Text(value, fontSize = 14.sp, modifier = Modifier.weight(1f))
    }
    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
}

private fun measurement(value: Measurement, unit: String): String = when (value) {
    is Measurement.Known -> "${NumberFormat.getNumberInstance().format(value.value)} $unit"
    Measurement.Unknown -> "Unknown"
    Measurement.NotApplicable -> "N/A"
}

fun failureMessage(kind: FailureKind): String = when (kind) {
    FailureKind.Storage -> "The archive could not be saved. Check available device storage."
    FailureKind.Network -> "The archive is out of reach. Check your connection and try again."
    FailureKind.Timeout -> "The archive took too long to respond. Try again in a moment."
    is FailureKind.Http -> if (kind.status == 404.toUShort()) "This record could not be found." else "The archive is temporarily unavailable."
    FailureKind.InvalidData -> "This record arrived in a format we could not read."
    FailureKind.InvalidRequest -> "This record cannot be requested."
}
