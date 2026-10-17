package dev.holocron.app

import androidx.compose.animation.*
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.rememberTextFieldState
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
import dev.holocron.core.CatalogCategory
import dev.holocron.core.FailureKind
import dev.holocron.core.PeoplePhase
import dev.holocron.core.PeopleSnapshot

@Composable
fun HolocronNavigation(endpoint: String? = null, archive: ArchiveController? = null, storageUnavailable: Boolean = false) {
    val navigation = rememberNavController()
    fun select(category: CatalogCategory) {
        val route = if (category == CatalogCategory.PEOPLE) "people" else "catalog/${category.name}"
        navigation.navigate(route) { popUpTo("people"); launchSingleTop = true }
    }
    CompositionLocalProvider(LocalArchive provides archive, LocalOpenArchive provides { navigation.navigate("saved") { launchSingleTop = true } }) {
    NavHost(navigation, startDestination = "people",
        enterTransition = { fadeIn(tween(240)) + scaleIn(initialScale = .96f, animationSpec = tween(300)) },
        exitTransition = { fadeOut(tween(180)) },
        popEnterTransition = { fadeIn(tween(220)) },
        popExitTransition = { fadeOut(tween(180)) + scaleOut(targetScale = .96f, animationSpec = tween(240)) }) {
        composable("saved") { archive?.let { SavedArchiveScreen(it, { navigation.popBackStack() }) { category, id ->
            navigation.navigate(if (category == CatalogCategory.PEOPLE) "person/$id" else "record/${category.name}/$id")
        } } }
        composable("people") {
            val model: PeopleViewModel = viewModel(factory = viewModelFactory { initializer { PeopleViewModel(createSavedStateHandle(), endpoint, archive?.session) } })
            val snapshot by model.controller.state.collectAsStateWithLifecycle()
            val query by model.query.collectAsStateWithLifecycle()
            PeopleScreen(snapshot, query, model::search, model.controller::refresh, model.controller::loadMore, model.controller::retry, onCategory = ::select) {
                navigation.navigate("person/$it") { launchSingleTop = true }
            }
        }
        composable("catalog/{category}") { entry ->
            val category = CatalogCategory.valueOf(requireNotNull(entry.arguments?.getString("category")))
            val model: CatalogViewModel = viewModel(factory = viewModelFactory { initializer { CatalogViewModel(createSavedStateHandle(), category, null, endpoint, archive?.session) } })
            val snapshot by model.controller.state.collectAsStateWithLifecycle()
            val query by model.query.collectAsStateWithLifecycle()
            CatalogScreen(category, snapshot, query, model::search, model.controller::refresh, model.controller::loadMore, model.controller::retry, ::select) {
                navigation.navigate("record/${category.name}/$it") { launchSingleTop = true }
            }
        }
        composable("record/{category}/{id}", arguments = listOf(navArgument("id") { type = NavType.IntType })) { entry ->
            val category = CatalogCategory.valueOf(requireNotNull(entry.arguments?.getString("category")))
            val id = requireNotNull(entry.arguments).getInt("id").toUInt()
            val model: CatalogViewModel = viewModel(factory = viewModelFactory { initializer { CatalogViewModel(createSavedStateHandle(), category, id, endpoint, archive?.session) } })
            val snapshot by model.controller.state.collectAsStateWithLifecycle()
            CatalogDetailScreen(category, snapshot, model.controller::refresh, model.controller::retry) { navigation.popBackStack() }
        }
        composable("person/{id}", arguments = listOf(navArgument("id") { type = NavType.IntType })) { entry ->
            val id = requireNotNull(entry.arguments).getInt("id").toUInt()
            val model: CharacterViewModel = viewModel(factory = viewModelFactory { initializer { CharacterViewModel(id, endpoint, archive?.session) } })
            Box {
                CharacterRoute(model.controller)
                FilledIconButton(onClick = { navigation.popBackStack() }, modifier = Modifier.statusBarsPadding().padding(start = 12.dp, top = 48.dp),
                    colors = IconButtonDefaults.filledIconButtonColors(containerColor = DesignTokens.Hero.background, contentColor = DesignTokens.Hero.text)) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to people")
                }
            }
        }
    }
    if (storageUnavailable) Text("Offline storage unavailable. Browsing uses live data.", modifier = Modifier.statusBarsPadding().padding(24.dp), color = MaterialTheme.colorScheme.error)
    }
}

@Composable
fun PeopleScreen(snapshot: PeopleSnapshot, query: String, onSearch: (String) -> Unit, onRefresh: () -> Unit,
                 onMore: () -> Unit, onRetry: () -> Unit, onCategory: (CatalogCategory) -> Unit = {}, onPerson: (UInt) -> Unit) {
    val failure = snapshot.phase as? PeoplePhase.Failed
    ArchiveScreen(CatalogCategory.PEOPLE, snapshot.people.map { ArchiveItem(it.id, it.name, "${it.filmCount} films · Born ${it.birthYear}") },
        snapshot.total, snapshot.hasMore, snapshot.phase == PeoplePhase.Ready,
        snapshot.phase == PeoplePhase.Loading || snapshot.phase == PeoplePhase.Refreshing || snapshot.phase == PeoplePhase.LoadingMore,
        failure?.kind, failure?.retryable == true, query, onSearch, onRefresh, onMore, onRetry, onCategory, onPerson, snapshot.cache)
}

data class ArchiveItem(val id: UInt, val name: String, val summary: String)

@Composable
fun ArchiveScreen(category: CatalogCategory, records: List<ArchiveItem>, total: UInt, hasMore: Boolean, ready: Boolean,
                  busy: Boolean, failure: FailureKind?, retryable: Boolean, query: String,
                  onSearch: (String) -> Unit, onRefresh: () -> Unit, onMore: () -> Unit, onRetry: () -> Unit,
                  onCategory: (CatalogCategory) -> Unit, onPerson: (UInt) -> Unit, cache: dev.holocron.core.CacheStatus = emptyCache()) {
    // Keep selection, composing text and IME updates in one synchronous native
    // state holder. Feeding edits back through StateFlow can replay stale text
    // when the keyboard connects. Rust observes the draft; it still debounces.
    val draft = rememberTextFieldState(initialText = query)
    val sendQuery by rememberUpdatedState(onSearch)
    LaunchedEffect(draft) { snapshotFlow { draft.text.toString() }.collect { sendQuery(it) } }
    val draftText = draft.text.toString()
    val keyboard = LocalSoftwareKeyboardController.current
    val largeText = LocalDensity.current.fontScale > 1.3f
    val list = rememberLazyListState()
    Surface(Modifier.fillMaxSize()) { Box {
        LazyColumn(state = list, modifier = Modifier.fillMaxSize().imePadding(),
            contentPadding = PaddingValues(bottom = 36.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            item(key = "hero") {
                Box(Modifier.fillMaxWidth().clip(RoundedCornerShape(bottomStart = DesignTokens.Shape.hero.dp, bottomEnd = DesignTokens.Shape.hero.dp))) {
                    Image(painterResource(category.artwork), null, Modifier.matchParentSize(), contentScale = ContentScale.Crop)
                    Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(DesignTokens.Hero.background.copy(alpha = .4f), DesignTokens.Hero.background.copy(alpha = .6f), DesignTokens.Hero.background))))
                    Column(Modifier.statusBarsPadding().padding(24.dp)) {
                        Text("HOLOCRON  ✦", color = DesignTokens.Hero.accent, fontSize = 12.sp, letterSpacing = 2.sp)
                        Spacer(Modifier.height(if (draftText.isEmpty()) 60.dp else 12.dp))
                        Text(if (draftText.isEmpty()) category.headline else "Find your story.", color = DesignTokens.Hero.text,
                            fontSize = if (draftText.isEmpty()) DesignTokens.Typography.hero.sp else 30.sp, lineHeight = 46.sp,
                            fontWeight = FontWeight.Medium, modifier = Modifier.semantics { heading() })
                        if (draftText.isEmpty()) Text(category.subtitle, color = DesignTokens.Hero.secondary, fontSize = 14.sp, modifier = Modifier.padding(top = 10.dp))
                    }
                }
            }
            item(key = "categories") { CategoryPicker(category, onCategory) }
            item(key = "saved") { ArchiveShortcut() }
            item(key = "search") {
                OutlinedTextField(state = draft, lineLimits = TextFieldLineLimits.SingleLine, label = { Text("Search ${category.title.lowercase()}") },
                    leadingIcon = { Icon(Icons.Outlined.Search, null) },
                    trailingIcon = { if (draftText.isNotEmpty()) IconButton(onClick = { draft.edit { replace(0, length, "") } }) { Icon(Icons.Outlined.Close, "Clear search") } },
                    shape = RoundedCornerShape(DesignTokens.Shape.control.dp), modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search, autoCorrectEnabled = false), onKeyboardAction = { keyboard?.hide() })
            }
            item(key = "heading") {
                Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(if (draftText.isEmpty()) "${category.title} of the galaxy" else "Search results", fontSize = DesignTokens.Typography.section.sp, modifier = Modifier.semantics { heading() })
                        Text(if (busy && records.isEmpty()) "Searching the archive…" else "${records.size} of ${total} records", color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp)
                    }
                    IconButton(onClick = onRefresh, enabled = !busy) { Icon(Icons.Outlined.Refresh, "Refresh ${category.title.lowercase()}") }
                }
            }
            item(key = "cache") { Box(Modifier.padding(horizontal = 24.dp)) { CacheNotice(cache) } }
            if (busy) item(key = "progress") { LinearProgressIndicator(Modifier.fillMaxWidth().padding(horizontal = 24.dp)) }
            items(records, key = { it.id.toString() }) { person ->
                Surface(onClick = { keyboard?.hide(); onPerson(person.id) }, color = MaterialTheme.colorScheme.surfaceContainer,
                    shape = RoundedCornerShape(DesignTokens.Shape.card.dp), modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) {
                    Row(Modifier.padding(18.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                        if (!largeText) Box(Modifier.size(48.dp).background(MaterialTheme.colorScheme.primary.copy(alpha = .1f), RoundedCornerShape(16.dp)), contentAlignment = Alignment.Center) {
                            Text(person.name.split(' ').take(2).mapNotNull { it.firstOrNull() }.joinToString(""), color = MaterialTheme.colorScheme.primary, fontSize = 17.sp)
                        }
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(person.name, fontSize = 18.sp, fontWeight = FontWeight.Medium)
                            Text(person.summary, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, tint = MaterialTheme.colorScheme.primary)
                    }
                }
            }
            if (ready && records.isEmpty()) item(key = "empty") {
                Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("No signals found", fontSize = 24.sp)
                    Text("Try another name. Even a few letters can lead to a story.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    OutlinedButton(onClick = { draft.edit { replace(0, length, "") } }) { Text("Explore all ${category.title.lowercase()}") }
                }
            }
            if (failure != null) item(key = "failure") {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(if (records.isEmpty()) "Signal interrupted" else "Couldn’t update the archive", fontSize = 22.sp)
                    Text(failureMessage(failure), color = MaterialTheme.colorScheme.onSurfaceVariant)
                    if (retryable) FilledTonalButton(onClick = onRetry) { Text("Try again") }
                }
            }
            if (ready && hasMore) item(key = "more") {
                OutlinedButton(onClick = onMore, modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp)) { Text("Load more ${category.title.lowercase()}") }
            }
            item { Spacer(Modifier.navigationBarsPadding()) }
        }
        Box(Modifier.fillMaxWidth().windowInsetsTopHeight(WindowInsets.statusBars).background(DesignTokens.Hero.background.copy(alpha = .9f)))
    } }
}
