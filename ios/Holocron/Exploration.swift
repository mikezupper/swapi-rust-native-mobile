import SwiftUI

extension ExplorationSnapshot {
    static var empty: Self { Self(featured: nil, recent: [], suggestions: [], ships: [], metrics: [], widget: []) }
}

struct RecordDestination: View {
    let category: CatalogCategory
    let id: UInt32
    var body: some View {
        if category == .people { CharacterDestination(id: id) } else { CatalogDestination(category: category, id: id) }
    }
}

struct RelatedRecords: View {
    let groups: [CatalogRelation]
    var body: some View {
        if !groups.isEmpty {
            VStack(alignment: .leading, spacing: 12) {
                Text("Connected stories").font(.title2).accessibilityAddTraits(.isHeader)
                Text("Follow a connection into the galaxy.").font(.subheadline)
                ForEach(groups, id: \.label) { group in
                    NavigationLink {
                        if group.ids.count == 1, let id = group.ids.first { RecordDestination(category: group.category, id: id) }
                        else { RelatedDestination(group: group) }
                    } label: {
                        HStack { Text("\(group.label) · \(group.ids.count)"); Spacer(); Image(systemName: "arrow.right") }
                            .frame(minHeight: 36).fixedSize(horizontal: false, vertical: true)
                    }.buttonStyle(.bordered)
                }
            }
        }
    }
}

struct RelatedDestination: View {
    let group: CatalogRelation
    @StateObject private var model: CatalogModel
    @Namespace private var transition
    init(group: CatalogRelation) {
        self.group = group
        _model = StateObject(wrappedValue: CatalogModel(category: group.category, relatedIds: group.ids))
    }
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                Text("Follow the story.").font(.largeTitle).accessibilityAddTraits(.isHeader)
                ForEach(model.snapshot.items, id: \.id) { record in
                    NavigationLink { RecordDestination(category: group.category, id: record.id).modifier(ArchiveZoom(id: record.id, namespace: transition)) } label: {
                        VStack(alignment: .leading, spacing: 8) {
                            Text(record.name).font(.title2)
                            Text(record.facts.prefix(2).map(\.value).joined(separator: " · ")).font(.subheadline)
                        }.frame(maxWidth: .infinity, alignment: .leading).padding(20)
                            .background(.quaternary, in: RoundedRectangle(cornerRadius: 24)).modifier(ArchiveZoomSource(id: record.id, namespace: transition))
                    }.buttonStyle(.plain)
                    if group.category == .starships { CompareShipButton(id: record.id) }
                }
                if model.snapshot.phase == .loading || model.snapshot.phase == .loadingMore { ProgressView().accessibilityLabel("Loading connections") }
                if case .failed = model.snapshot.phase {
                    Text("Some connections are unavailable. Connect to the network or download this collection for offline use.")
                    Button("Try again", action: model.retry).buttonStyle(.borderedProminent)
                }
                if model.snapshot.hasMore && model.snapshot.phase == .ready { Button("More connections", action: model.loadMore).buttonStyle(.bordered) }
            }.padding(24)
        }.safeAreaInset(edge: .bottom) { ComparisonTray() }.modifier(ExplorationPalette())
            .navigationTitle("Connected \(group.category.title.lowercased())").navigationBarTitleDisplayMode(.inline)
    }
}

struct CompareShipButton: View {
    let id: UInt32
    @ObservedObject private var archive = ArchiveModel.shared
    private var selected: Bool { archive.snapshot.exploration.ships.contains { $0.id == id } }
    var body: some View {
        if archive.session != nil {
            Button { try? archive.session?.selectShip(id: id, selected: !selected) } label: {
                Label(selected ? "Remove from comparison" : (archive.snapshot.exploration.ships.count == 2 ? "Comparison full · 2 ships" : "Compare ship"), systemImage: selected ? "minus.circle" : "plus.circle")
                .frame(minHeight: 36)
            }.buttonStyle(.bordered).disabled(!selected && archive.snapshot.exploration.ships.count == 2)
                .sensoryFeedback(.selection, trigger: selected).accessibilityIdentifier("compare-\(id)")
        }
    }
}

struct ComparisonTray: View {
    @ObservedObject private var archive = ArchiveModel.shared
    @State private var show = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorScheme) private var appearance
    private var ships: [CatalogEntry] { archive.snapshot.exploration.ships }
    var body: some View {
        VStack(spacing: 0) {
            if !ships.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("SHIP COMPARISON · \(ships.count)/2").font(.caption).tracking(1.5)
                    ForEach(ships, id: \.id) { ship in
                        HStack {
                            Text(ship.name).font(.subheadline).frame(maxWidth: .infinity, alignment: .leading)
                            Button { try? archive.session?.selectShip(id: ship.id, selected: false) } label: { Text("Remove").font(.subheadline).frame(minWidth: 44, minHeight: 44) }
                                .buttonStyle(.plain).accessibilityLabel("Remove \(ship.name)")
                        }
                    }
                    Button { show = true } label: { Text(ships.count == 2 ? "Compare starships" : "Choose one more starship").frame(minHeight: 36) }
                        .buttonStyle(.borderedProminent).disabled(ships.count != 2).frame(maxWidth: .infinity)
                }.padding(.horizontal, 24).padding(.vertical, 12)
                    .frame(maxWidth: .infinity)
                    .background(appearance == .dark ? DesignTokens.Dark.surface : DesignTokens.Light.surface)
                    .transition(reduceMotion ? .identity : .move(edge: .bottom).combined(with: .opacity))
            }
        }.modifier(ExplorationPalette())
            .animation(reduceMotion ? nil : .easeInOut(duration: 0.25), value: ships.count)
            .sheet(isPresented: $show) { NavigationStack { ComparisonScreen().toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { show = false } } } } }
    }
}

struct ComparisonScreen: View {
    @State private var revealed = false
    @ObservedObject private var archive = ArchiveModel.shared
    @Environment(\.colorScheme) private var appearance
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var state: ExplorationSnapshot { archive.snapshot.exploration }
    private var accent: Color { appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent }
    private var warm: Color { appearance == .dark ? DesignTokens.Dark.warm : DesignTokens.Light.warm }
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Text("Choose your ride.").font(.largeTitle).accessibilityAddTraits(.isHeader)
                if state.ships.count != 2 { Text("Select two starships from their collection or detail screens.") }
                ForEach(Array(state.ships.enumerated()), id: \.element.id) { index, ship in
                    VStack(alignment: .leading, spacing: 8) {
                        Text(index == 0 ? "SHIP A" : "SHIP B").font(.caption).tracking(2)
                        Text(ship.name).font(.title2)
                        Text(ship.facts.first { $0.label == "Model" }?.value ?? "").font(.subheadline)
                    }.frame(maxWidth: .infinity, alignment: .leading).padding(20)
                        .background((index == 0 ? accent : warm).opacity(0.12), in: RoundedRectangle(cornerRadius: 24))
                }
                ForEach(state.metrics, id: \.label) { metric in
                    VStack(alignment: .leading, spacing: 12) {
                        Text(metric.label).font(.title2).accessibilityAddTraits(.isHeader)
                        ForEach(Array(metric.values.enumerated()), id: \.offset) { index, value in
                            if index < state.ships.count {
                                Text("\(state.ships[index].name): \(value.display)").font(.subheadline).fixedSize(horizontal: false, vertical: true)
                                if let fraction = value.fraction {
                                    ProgressView(value: revealed ? fraction : 0).tint(index == 0 ? accent : warm).accessibilityHidden(true)
                                        .animation(reduceMotion ? nil : .easeInOut(duration: 0.6), value: revealed)
                                        .animation(reduceMotion ? nil : .easeInOut(duration: 0.6), value: fraction)
                                }
                            }
                        }
                        if metric.lowerIsBetter { Text("Lower hyperdrive ratings are faster.").font(.caption) }
                    }
                }
                Text("Bars show relative values, not an overall score. Unknown values and crew ranges keep their original meaning.").font(.caption)
            }.padding(24)
        }.modifier(ExplorationPalette()).navigationTitle("Starship comparison").navigationBarTitleDisplayMode(.inline).onAppear { revealed = true }
    }
}

struct DiscoveryScreen: View {
    @ObservedObject private var archive = ArchiveModel.shared
    @Environment(\.dismiss) private var dismiss
    @Namespace private var transition
    private var state: ExplorationSnapshot { archive.snapshot.exploration }
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    Text("Your next galactic detour.").font(.largeTitle).accessibilityAddTraits(.isHeader)
                    Text(archive.snapshot.offline ? "Discoveries from your saved galaxy." : "A familiar world. An unexpected connection.")
                    if let featured = state.featured {
                        NavigationLink { RecordDestination(category: featured.category, id: featured.entry.id).modifier(ArchiveZoom(id: featured.entry.id, namespace: transition)) } label: {
                            VStack(alignment: .leading, spacing: 12) {
                                Text("FEATURED WORLD").font(.caption).tracking(2).foregroundStyle(DesignTokens.Hero.accent).padding(8).background(DesignTokens.Hero.background.opacity(0.92), in: RoundedRectangle(cornerRadius: 8))
                                Spacer(minLength: 100)
                                Text(featured.entry.name).font(.largeTitle)
                                Text(featured.entry.facts.prefix(2).map(\.value).joined(separator: " · ")).font(.subheadline)
                                Text("Explore world →").foregroundStyle(DesignTokens.Hero.accent)
                            }.padding(24).frame(maxWidth: .infinity, alignment: .leading).foregroundStyle(DesignTokens.Hero.text)
                                .background {
                                    GeometryReader { geometry in
                                        Image(featured.category.artwork).resizable().scaledToFill().frame(width: geometry.size.width, height: geometry.size.height).clipped().accessibilityHidden(true)
                                            .overlay(LinearGradient(colors: [.clear, DesignTokens.Hero.background], startPoint: .top, endPoint: .bottom))
                                    }.modifier(ArchiveZoomSource(id: featured.entry.id, namespace: transition))
                                }.clipShape(RoundedRectangle(cornerRadius: 28))
                        }.buttonStyle(.plain)
                    } else {
                        Text(archive.snapshot.offline ? "Save a planet to feature it here." : "Preparing your first discovery…")
                        Button("Find a world") { try? archive.session?.discover() }.buttonStyle(.bordered)
                    }
                    if archive.snapshot.error != nil { Text("Discovery could not update. Saved records remain available.") }
                    if !state.recent.isEmpty {
                        Text("Recently explored").font(.title2).accessibilityAddTraits(.isHeader)
                        ForEach(state.recent, id: \.discoveryIdentity) { row($0) }
                    }
                    Text("Continue exploring").font(.title2).accessibilityAddTraits(.isHeader)
                    if state.suggestions.isEmpty { Text("Open a few records to build your own trail through the stars.") }
                    ForEach(state.suggestions, id: \.discoveryIdentity) { row($0) }
                    NavigationLink { WidgetPreviewScreen() } label: { Text("Preview home-screen widget").frame(minHeight: 44) }
                    Text("Add the Holocron widget from your home screen’s widget gallery for a favorite and a saved discovery.").font(.caption)
                }.padding(24)
            }.safeAreaInset(edge: .bottom) { ComparisonTray() }.modifier(ExplorationPalette())
                .navigationTitle("Discover").navigationBarTitleDisplayMode(.inline)
                .toolbar { ToolbarItem(placement: .confirmationAction) { Button { dismiss() } label: { Image(systemName: "xmark") }.accessibilityLabel("Close discovery") } }
        }.modifier(ExplorationPalette())
            .task { try? archive.session?.discover() }
    }
    private func row(_ saved: SavedEntry) -> some View {
        NavigationLink { RecordDestination(category: saved.category, id: saved.entry.id) } label: {
            HStack(spacing: 14) {
                Image(saved.category.artwork).resizable().scaledToFill().frame(width: 56, height: 56).clipShape(RoundedRectangle(cornerRadius: 14)).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 6) { Text(saved.entry.name).font(.headline); Text(saved.category.title).font(.caption) }.frame(maxWidth: .infinity, alignment: .leading)
                Image(systemName: "arrow.right")
            }.padding(16).background(.quaternary, in: RoundedRectangle(cornerRadius: 20))
        }.buttonStyle(.plain)
    }
}

extension SavedEntry { var discoveryIdentity: String { "\(category.title)/\(entry.id)" } }

struct FilmExperience: View {
    let title: String
    let crawl: String
    @State private var cinematic = false
    @State private var playing = false
    @State private var elapsed: Double = 0
    @State private var started = Date()
    @State private var textHeight: CGFloat = 0
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.dynamicTypeSize) private var typeSize
    @Environment(\.scenePhase) private var scenePhase
    private var duration: Double { min(160, max(35, Double(crawl.count) / 12)) }
    private var reading: Bool { !cinematic || reduceMotion || typeSize.isAccessibilitySize }
    private func position(_ date: Date) -> Double { min(duration, elapsed + (playing ? max(0, date.timeIntervalSince(started)) : 0)) }
    private func pause() { elapsed = position(Date()); playing = false }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Opening crawl").font(.title2).accessibilityAddTraits(.isHeader)
            Button(cinematic ? "Read opening crawl" : "Cinematic opening crawl") { pause(); cinematic.toggle() }.buttonStyle(.bordered)
            if reading {
                if cinematic { Text("Reading mode follows your text and motion preferences.").font(.caption) }
                Text(crawl).font(.body).lineSpacing(6).textSelection(.enabled)
            } else {
                TimelineView(.animation(minimumInterval: 1.0 / 30, paused: !playing || scenePhase != .active)) { context in
                    let time = position(context.date)
                    VStack(spacing: 16) {
                        ZStack(alignment: .top) {
                            Image(CatalogCategory.films.artwork).resizable().scaledToFill().opacity(0.18)
                            Text(title + "\n\n" + crawl).font(.title2.weight(.semibold)).multilineTextAlignment(.center).lineSpacing(8)
                                .foregroundStyle(DesignTokens.Dark.warm).padding(.horizontal, 30)
                                .fixedSize(horizontal: false, vertical: true)
                                .onGeometryChange(for: CGFloat.self, of: { $0.size.height }) { textHeight = $0 }
                                .rotation3DEffect(.degrees(22), axis: (x: 1, y: 0, z: 0), perspective: 0.2)
                                .offset(y: 360 - (360 + textHeight) * time / duration)
                        }.frame(height: 360).frame(maxWidth: .infinity).background(DesignTokens.Hero.background)
                            .clipped().clipShape(RoundedRectangle(cornerRadius: 24))
                            .accessibilityElement(children: .ignore).accessibilityLabel("Animated opening crawl for \(title). Use Read opening crawl for the complete text.")
                        HStack {
                            Button(playing && time < duration ? "Pause crawl" : (time >= duration ? "Replay crawl" : "Play crawl")) {
                                if playing && time < duration { pause() } else { if time >= duration { elapsed = 0 }; started = Date(); playing = true }
                            }.buttonStyle(.borderedProminent)
                            Button("Restart crawl") { playing = false; elapsed = 0 }.buttonStyle(.bordered)
                        }
                        Slider(value: Binding(get: { time / duration }, set: { playing = false; elapsed = $0 * duration })).accessibilityLabel("Crawl progress")
                    }.onChange(of: time >= duration) { _, ended in if ended { elapsed = duration; playing = false } }
                }
            }
        }.onChange(of: scenePhase) { _, phase in if phase != .active { pause() } }
            .onChange(of: reduceMotion) { _, _ in pause() }.onChange(of: typeSize) { _, _ in pause() }.onDisappear { pause() }
    }
}

struct WidgetPreviewScreen: View {
    @ObservedObject private var archive = ArchiveModel.shared
    private var cards: [WidgetCard] { archive.snapshot.exploration.widget.map { saved in
        WidgetCard(name: saved.entry.name, subtitle: "\(saved.favorite ? "Favorite" : "Saved discovery") · \(saved.category.title)", url: recordLink(category: saved.category, id: saved.entry.id), artwork: saved.category.artwork)
    } }
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Text("Your galaxy, at a glance.").font(.largeTitle)
                WidgetContentView(cards: cards, medium: true).padding(20).frame(minHeight: 180).background(DesignTokens.Hero.background).clipShape(RoundedRectangle(cornerRadius: 24))
                Text("This is the same content view used by the WidgetKit extension. Add Holocron from the home-screen widget gallery.").font(.subheadline)
            }.padding(24)
        }.navigationTitle("Widget preview").navigationBarTitleDisplayMode(.inline)
    }
}

/// Keep system presentations and pinned controls on the same native design palette.
private struct ExplorationPalette: ViewModifier {
    @Environment(\.colorScheme) private var appearance
    func body(content: Content) -> some View {
        content.background(appearance == .dark ? DesignTokens.Dark.background : DesignTokens.Light.background)
            .foregroundStyle(appearance == .dark ? DesignTokens.Dark.text : DesignTokens.Light.text)
            .tint(appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent)
    }
}
