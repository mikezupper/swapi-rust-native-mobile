package dev.holocron.app

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.createSavedStateHandle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavType
import androidx.navigation.compose.*
import androidx.navigation.navArgument
import dev.holocron.core.PeoplePhase
import dev.holocron.core.PeopleSnapshot

@Composable
fun HolocronNavigation(endpoint: String? = null) {
    val navigation = rememberNavController()
    NavHost(navigation, startDestination = "people") {
        composable("people") {
            val model: PeopleViewModel = viewModel(factory = viewModelFactory { initializer { PeopleViewModel(createSavedStateHandle(), endpoint) } })
            val snapshot by model.controller.state.collectAsStateWithLifecycle()
            val query by model.query.collectAsStateWithLifecycle()
            PeopleScreen(snapshot, query, model::search, model.controller::refresh, model.controller::loadMore, model.controller::retry) {
                navigation.navigate("person/$it") { launchSingleTop = true }
            }
        }
        composable("person/{id}", arguments = listOf(navArgument("id") { type = NavType.IntType })) { entry ->
            val id = requireNotNull(entry.arguments).getInt("id").toUInt()
            val model: CharacterViewModel = viewModel(factory = viewModelFactory { initializer { CharacterViewModel(id, endpoint) } })
            Box {
                CharacterRoute(model.controller)
                FilledIconButton(onClick = { navigation.popBackStack() }, modifier = Modifier.statusBarsPadding().padding(start = 12.dp, top = 48.dp),
                    colors = IconButtonDefaults.filledIconButtonColors(containerColor = DesignTokens.Hero.background, contentColor = DesignTokens.Hero.text)) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to people")
                }
            }
        }
    }
}

@Composable
fun PeopleScreen(snapshot: PeopleSnapshot, query: String, onSearch: (String) -> Unit, onRefresh: () -> Unit,
                 onMore: () -> Unit, onRetry: () -> Unit, onPerson: (UInt) -> Unit) {
    val keyboard = LocalSoftwareKeyboardController.current
    val largeText = LocalDensity.current.fontScale > 1.3f
    val list = rememberLazyListState()
    val busy = snapshot.phase == PeoplePhase.Loading || snapshot.phase == PeoplePhase.Refreshing || snapshot.phase == PeoplePhase.LoadingMore
    Surface(Modifier.fillMaxSize()) { Box {
        LazyColumn(state = list, modifier = Modifier.fillMaxSize().imePadding(),
            contentPadding = PaddingValues(bottom = 36.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            item(key = "hero") {
                Box(Modifier.fillMaxWidth().clip(RoundedCornerShape(bottomStart = DesignTokens.Shape.hero.dp, bottomEnd = DesignTokens.Shape.hero.dp))) {
                    Image(painterResource(R.drawable.desert_orbit), null, Modifier.matchParentSize(), contentScale = ContentScale.Crop)
                    Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(DesignTokens.Hero.background.copy(alpha = .4f), DesignTokens.Hero.background.copy(alpha = .6f), DesignTokens.Hero.background))))
                    Column(Modifier.statusBarsPadding().padding(24.dp)) {
                        Text("HOLOCRON  ✦", color = DesignTokens.Hero.accent, fontSize = 12.sp, letterSpacing = 2.sp)
                        Spacer(Modifier.height(if (query.isEmpty()) 60.dp else 12.dp))
                        Text(if (query.isEmpty()) "A galaxy of\nstories." else "Find your story.", color = DesignTokens.Hero.text,
                            fontSize = if (query.isEmpty()) DesignTokens.Typography.hero.sp else 30.sp, lineHeight = 46.sp,
                            fontWeight = FontWeight.Medium, modifier = Modifier.semantics { heading() })
                        if (query.isEmpty()) Text("Discover the people behind the legends.", color = DesignTokens.Hero.secondary, fontSize = 14.sp, modifier = Modifier.padding(top = 10.dp))
                    }
                }
            }
            item(key = "search") {
                OutlinedTextField(query, onSearch, singleLine = true, label = { Text("Search people") },
                    leadingIcon = { Icon(Icons.Outlined.Search, null) },
                    trailingIcon = { if (query.isNotEmpty()) IconButton(onClick = { onSearch("") }) { Icon(Icons.Outlined.Close, "Clear search") } },
                    shape = RoundedCornerShape(DesignTokens.Shape.control.dp), modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search), keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }))
            }
            item(key = "heading") {
                Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(if (query.isEmpty()) "People of the galaxy" else "Search results", fontSize = DesignTokens.Typography.section.sp, modifier = Modifier.semantics { heading() })
                        Text(if (busy && snapshot.people.isEmpty()) "Searching the archive…" else "${snapshot.people.size} of ${snapshot.total} records", color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp)
                    }
                    IconButton(onClick = onRefresh, enabled = !busy) { Icon(Icons.Outlined.Refresh, "Refresh people") }
                }
            }
            if (busy) item(key = "progress") { LinearProgressIndicator(Modifier.fillMaxWidth().padding(horizontal = 24.dp)) }
            items(snapshot.people, key = { it.id.toString() }) { person ->
                Surface(onClick = { keyboard?.hide(); onPerson(person.id) }, color = MaterialTheme.colorScheme.surfaceContainer,
                    shape = RoundedCornerShape(DesignTokens.Shape.card.dp), modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) {
                    Row(Modifier.padding(18.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                        if (!largeText) Box(Modifier.size(48.dp).background(MaterialTheme.colorScheme.primary.copy(alpha = .1f), RoundedCornerShape(16.dp)), contentAlignment = Alignment.Center) {
                            Text(person.name.split(' ').take(2).mapNotNull { it.firstOrNull() }.joinToString(""), color = MaterialTheme.colorScheme.primary, fontSize = 17.sp)
                        }
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(person.name, fontSize = 18.sp, fontWeight = FontWeight.Medium)
                            Text("${person.filmCount} films · Born ${person.birthYear}", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, tint = MaterialTheme.colorScheme.primary)
                    }
                }
            }
            if (snapshot.phase == PeoplePhase.Ready && snapshot.people.isEmpty()) item(key = "empty") {
                Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("No signals found", fontSize = 24.sp)
                    Text("Try another name. Even a few letters can lead to a story.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    OutlinedButton(onClick = { onSearch("") }) { Text("Explore all people") }
                }
            }
            val failure = snapshot.phase as? PeoplePhase.Failed
            if (failure != null) item(key = "failure") {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(if (snapshot.people.isEmpty()) "Signal interrupted" else "Couldn’t update the archive", fontSize = 22.sp)
                    Text(failureMessage(failure.kind), color = MaterialTheme.colorScheme.onSurfaceVariant)
                    if (failure.retryable) FilledTonalButton(onClick = onRetry) { Text("Try again") }
                }
            }
            if (snapshot.phase == PeoplePhase.Ready && snapshot.hasMore) item(key = "more") {
                OutlinedButton(onClick = onMore, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) { Text("Load more people") }
            }
            item { Spacer(Modifier.navigationBarsPadding()) }
        }
        Box(Modifier.fillMaxWidth().windowInsetsTopHeight(WindowInsets.statusBars).background(DesignTokens.Hero.background.copy(alpha = .9f)))
    } }
}
