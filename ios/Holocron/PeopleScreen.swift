import SwiftUI

struct CharacterDestination: View {
    @StateObject private var model: CharacterModel
    init(id: UInt32) { _model = StateObject(wrappedValue: CharacterModel(personId: id)) }
    var body: some View { CharacterScreen(model: model).toolbar(.visible, for: .navigationBar) }
}

struct PeopleScreen: View {
    @ObservedObject var model: PeopleModel
    var onCategory: (CatalogCategory) -> Void = { _ in }
    @State private var query = ""
    @FocusState private var searching: Bool
    @Environment(\.colorScheme) private var appearance
    @Environment(\.dynamicTypeSize) private var typeSize
    @ScaledMetric(relativeTo: .largeTitle) private var heroSize = DesignTokens.Typography.hero
    private var background: Color { appearance == .dark ? DesignTokens.Dark.background : DesignTokens.Light.background }
    private var foreground: Color { appearance == .dark ? DesignTokens.Dark.text : DesignTokens.Light.text }
    private var secondary: Color { appearance == .dark ? DesignTokens.Dark.secondary : DesignTokens.Light.secondary }
    private var surface: Color { appearance == .dark ? DesignTokens.Dark.surface : DesignTokens.Light.surface }
    private var onAccent: Color { appearance == .dark ? DesignTokens.Dark.onAccent : DesignTokens.Light.onAccent }
    private var accent: Color { appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent }
    private var busy: Bool {
        switch model.snapshot.phase { case .loading, .refreshing, .loadingMore: return true; default: return false }
    }
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    hero
                    CategoryPicker(selected: .people, onSelect: onCategory)
                    VStack(spacing: 20) {
                        HStack(spacing: 12) {
                            Image(systemName: "magnifyingglass").foregroundStyle(accent)
                            TextField("Search people", text: $query, prompt: Text("Search people").foregroundStyle(secondary), axis: .vertical).focused($searching)
                                .textInputAutocapitalization(.never).autocorrectionDisabled()
                                .submitLabel(.search).onSubmit { searching = false }
                                .accessibilityIdentifier("people-search")
                            if !query.isEmpty {
                                Button { query = "" } label: { Image(systemName: "xmark.circle.fill").frame(minWidth: 32, minHeight: 44) }
                                    .accessibilityLabel("Clear search")
                            }
                        }
                        .padding(.horizontal, 16).padding(.vertical, 6)
                        .background(surface, in: RoundedRectangle(cornerRadius: DesignTokens.Shape.control))
                        HStack {
                            VStack(alignment: .leading, spacing: 6) {
                                Text(query.isEmpty ? "People of the galaxy" : "Search results").font(.title2).accessibilityAddTraits(.isHeader)
                                Text(busy && model.snapshot.people.isEmpty ? "Searching the archive…" : "\(model.snapshot.people.count) of \(model.snapshot.total) records")
                                    .font(.subheadline).foregroundStyle(foreground)
                            }.frame(maxWidth: .infinity, alignment: .leading)
                            Button(action: model.refresh) { Image(systemName: "arrow.clockwise").frame(width: 44, height: 44) }
                                .accessibilityLabel("Refresh people").disabled(busy)
                        }
                        if busy { ProgressView().frame(maxWidth: .infinity).accessibilityLabel("Loading people") }
                        ForEach(model.snapshot.people, id: \.id) { person in
                            NavigationLink(value: person.id) { personRow(person) }
                                .buttonStyle(.plain).accessibilityIdentifier("person-\(person.id)")
                        }
                        if model.snapshot.phase == .ready && model.snapshot.people.isEmpty {
                            VStack(alignment: .leading, spacing: 12) {
                                Text("No signals found").font(.title2)
                                Text("Try another name. Even a few letters can lead to a story.").foregroundStyle(secondary)
                                Button("Explore all people") { query = "" }.buttonStyle(.bordered)
                            }.frame(maxWidth: .infinity, alignment: .leading)
                        }
                        if case .failed(let kind, let retryable) = model.snapshot.phase {
                            VStack(alignment: .leading, spacing: 12) {
                                Text(model.snapshot.people.isEmpty ? "Signal interrupted" : "Couldn’t update the archive").font(.title2)
                                Text(message(kind)).foregroundStyle(secondary)
                                if retryable { Button("Try again", action: model.retry).buttonStyle(.borderedProminent) }
                            }.frame(maxWidth: .infinity, alignment: .leading)
                        }
                        if model.snapshot.phase == .ready && model.snapshot.hasMore {
                            Button("Load more people", action: model.loadMore).buttonStyle(.bordered).frame(maxWidth: .infinity)
                        }
                    }.padding(.horizontal, DesignTokens.Space.lg).padding(.bottom, 32)
                }
            }
            .background(background).foregroundStyle(foreground)
            .scrollDismissesKeyboard(.interactively)
            .onAppear { query = model.snapshot.query }
            .onChange(of: query) { _, value in
                // A vertically growing field keeps large text legible. Treat its
                // return key as Search instead of retaining a newline in the draft.
                if value.contains("\n") {
                    query = value.replacingOccurrences(of: "\n", with: " ").trimmingCharacters(in: .whitespacesAndNewlines)
                    searching = false
                } else { model.search(value) }
            }
            .navigationDestination(for: UInt32.self) { id in CharacterDestination(id: id) }
            .toolbar(.hidden, for: .navigationBar)
        }.tint(accent)
    }
    private var hero: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("HOLOCRON  ✦").font(.caption.weight(.semibold)).tracking(2).foregroundStyle(DesignTokens.Hero.accent)
            Spacer(minLength: query.isEmpty ? 54 : 4)
            Text(query.isEmpty ? "A galaxy of\nstories." : "Find your story.")
                .font(query.isEmpty ? .system(size: heroSize, weight: .medium) : .largeTitle)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityAddTraits(.isHeader)
            if query.isEmpty { Text("Discover the people behind the legends.").font(.subheadline).foregroundStyle(DesignTokens.Hero.secondary) }
        }
        .padding(DesignTokens.Space.lg).frame(maxWidth: .infinity, alignment: .leading)
        .foregroundStyle(DesignTokens.Hero.text)
        .background {
            GeometryReader { geometry in
                Image("desert_orbit").resizable().scaledToFill().frame(width: geometry.size.width, height: geometry.size.height).clipped()
                    .overlay(LinearGradient(colors: [DesignTokens.Hero.background.opacity(0.4), DesignTokens.Hero.background.opacity(0.6), DesignTokens.Hero.background], startPoint: .top, endPoint: .bottom))
            }
        }
        .clipShape(UnevenRoundedRectangle(bottomLeadingRadius: DesignTokens.Shape.hero, bottomTrailingRadius: DesignTokens.Shape.hero))
    }
    private func personRow(_ person: Character) -> some View {
        HStack(spacing: 16) {
            if !typeSize.isAccessibilitySize {
                Text(person.name.split(separator: " ").prefix(2).compactMap(\.first).map(String.init).joined())
                    .font(.headline.weight(.semibold)).foregroundStyle(onAccent).frame(width: 48, height: 48)
                    .background(accent, in: RoundedRectangle(cornerRadius: 16)).accessibilityHidden(true)
            }
            VStack(alignment: .leading, spacing: 6) {
                Text(person.name).font(.headline.weight(.medium)).fixedSize(horizontal: false, vertical: true)
                Text("\(person.filmCount) films · Born \(person.birthYear)").font(.caption).foregroundStyle(secondary)
            }.frame(maxWidth: .infinity, alignment: .leading)
            Image(systemName: "chevron.right").font(.caption).foregroundStyle(accent).accessibilityHidden(true)
        }.padding(18).background(surface, in: RoundedRectangle(cornerRadius: DesignTokens.Shape.card))
    }
    private func message(_ kind: FailureKind) -> String {
        switch kind {
        case .network: return "The archive is out of reach. Check your connection and try again."
        case .timeout: return "The archive took too long to respond. Try again in a moment."
        case .http: return "The archive is temporarily unavailable."
        case .invalidData: return "The archive arrived in a format we could not read."
        case .invalidRequest: return "This search cannot be requested."
        }
    }
}
