import SwiftUI
import Combine

extension CacheStatus {
    static var empty: Self { Self(savedAt: nil, fromCache: false, offline: false, refreshFailed: false, savingFailed: false) }
}
extension CharacterSnapshot {
    init(revision: UInt64, state: CharacterState) { self.init(revision: revision, state: state, cache: .empty) }
}
extension PeopleSnapshot {
    init(revision: UInt64, query: String, people: [Character], total: UInt32, hasMore: Bool, phase: PeoplePhase) {
        self.init(revision: revision, query: query, people: people, total: total, hasMore: hasMore, phase: phase, cache: .empty)
    }
}
extension CatalogSnapshot {
    init(revision: UInt64, query: String, items: [CatalogEntry], total: UInt32, hasMore: Bool, phase: CatalogPhase) {
        self.init(revision: revision, query: query, items: items, total: total, hasMore: hasMore, phase: phase, cache: .empty)
    }
}

@MainActor final class ArchiveModel: ObservableObject {
    static let shared = ArchiveModel()
    @Published private(set) var snapshot = ArchiveSnapshot(revision: 0, query: "", favoritesOnly: false, offline: false, entries: [], favorites: [], collections: [], download: .idle, error: nil)
    @Published private(set) var unavailable = false
    private(set) var session: ArchiveSession?
    private var subscription: Subscription?
    private var stopped = false
    init(core: ArchiveSession? = nil) {
        do {
            if let core { session = core } else {
                #if DEBUG
                let args = ProcessInfo.processInfo.arguments
                // Existing fixture tests remain stateless unless they explicitly test persistence.
                if args.contains("--endpoint") && !args.contains("--archive") { return }
                #endif
                let folder = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true).appendingPathComponent("Holocron", isDirectory: true)
                try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
                #if DEBUG
                // UI-test resets must never erase the production archive.
                let databaseName = args.contains("--endpoint") ? "archive-fixture.sqlite" : "archive.sqlite"
                #else
                let databaseName = "archive.sqlite"
                #endif
                let database = folder.appendingPathComponent(databaseName)
                #if DEBUG
                if args.contains("--endpoint") && args.contains("--archive-reset") {
                    for suffix in ["", "-wal", "-shm"] { try? FileManager.default.removeItem(atPath: database.path + suffix) }
                }
                if let index = args.firstIndex(of: "--endpoint"), args.count > index + 1 {
                    session = try ArchiveSession.withEndpoint(storagePath: database.path, baseUrl: args[index + 1])
                } else { session = try ArchiveSession(storagePath: database.path) }
                #else
                session = try ArchiveSession(storagePath: database.path)
                #endif
            }
            subscription = try session?.subscribe(listener: ArchiveStateListener(owner: self))
        } catch { unavailable = true }
    }
    func receive(_ value: ArchiveSnapshot) { guard !stopped, value.revision > snapshot.revision else { return }; snapshot = value }
    func isFavorite(_ category: CatalogCategory, _ id: UInt32) -> Bool { snapshot.favorites.contains { $0.category == category && $0.id == id } }
    func favorite(_ category: CatalogCategory, _ id: UInt32) { try? session?.setFavorite(category: category, id: id, value: !isFavorite(category, id)) }
    func search(_ value: String) { try? session?.search(query: value) }
    func favoritesOnly(_ value: Bool) { try? session?.setFavoritesOnly(value: value) }
    func offline(_ value: Bool) { try? session?.setOffline(value: value) }
    func download(_ category: CatalogCategory) { try? session?.download(category: category) }
    func cancelDownload() { try? session?.cancelDownload() }
    func clearCache() { try? session?.clearCache() }
    func stop() { stopped = true; subscription?.cancel(); subscription = nil; session?.shutdown(); session = nil }
    deinit { subscription?.cancel(); session?.shutdown() }
}
@MainActor private final class ArchiveStateListener: ArchiveListener {
    private weak var owner: ArchiveModel?
    init(owner: ArchiveModel) { self.owner = owner }
    nonisolated func onState(snapshot: ArchiveSnapshot) throws { Task { @MainActor [weak self] in self?.owner?.receive(snapshot) } }
}

struct ArchiveShortcut: View {
    @ObservedObject private var archive = ArchiveModel.shared
    @State private var showArchive = false
    var body: some View {
        if archive.session != nil {
            Button { showArchive = true } label: {
                Label(archive.snapshot.offline ? "Offline mode · Open saved archive" : "Saved archive · \(archive.snapshot.favorites.count) \(archive.snapshot.favorites.count == 1 ? "favorite" : "favorites")", systemImage: "square.stack.3d.up")
                    .font(.subheadline).frame(maxWidth: .infinity, minHeight: 36)
            }.buttonStyle(.bordered).padding(.horizontal, 24).accessibilityIdentifier("open-saved")
                .sheet(isPresented: $showArchive) { SavedArchiveScreen(archive: archive) }
        } else if archive.unavailable {
            Text("Offline storage unavailable. Browsing uses live data.").font(.caption).padding(.horizontal, 24)
        }
    }
}
struct CacheNotice: View {
    let cache: CacheStatus
    @Environment(\.colorScheme) private var appearance
    private var secondary: Color { appearance == .dark ? DesignTokens.Dark.secondary : DesignTokens.Light.secondary }
    private var message: String {
        if cache.savingFailed { return "Live data · Could not save for offline use" }
        let date = cache.savedAt.map { Date(timeIntervalSince1970: Double($0)).formatted(date: .abbreviated, time: .shortened) } ?? ""
        if cache.refreshFailed { return "Saved copy · Refresh unavailable · \(date)" }
        if cache.offline { return "Offline copy · Saved \(date)" }
        return (cache.fromCache ? "Saved copy · Updated " : "Available offline · Updated ") + date
    }
    var body: some View {
        if cache.savedAt != nil || cache.savingFailed { Text(message).font(.caption).foregroundStyle(secondary).frame(maxWidth: .infinity, alignment: .leading) }
    }
}
struct FavoriteButton: View {
    let category: CatalogCategory
    let id: UInt32
    @ObservedObject private var archive = ArchiveModel.shared
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var saved: Bool { archive.isFavorite(category, id) }
    var body: some View {
        if archive.session != nil {
            Button { archive.favorite(category, id) } label: {
                Label(saved ? "Remove favorite" : "Add favorite", systemImage: saved ? "star.fill" : "star")
                    .contentTransition(reduceMotion ? .identity : .symbolEffect(.replace, options: .nonRepeating))
            }.buttonStyle(.bordered).sensoryFeedback(.selection, trigger: saved)
                .animation(reduceMotion ? nil : .easeInOut(duration: 0.2), value: saved)
                .accessibilityIdentifier("favorite-record")
        }
    }
}
struct ArchiveZoom: ViewModifier {
    let id: UInt32
    let namespace: Namespace.ID
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ViewBuilder func body(content: Content) -> some View {
        if reduceMotion { content } else { content.navigationTransition(.zoom(sourceID: id, in: namespace)) }
    }
}

struct SavedArchiveScreen: View {
    @ObservedObject var archive: ArchiveModel
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var appearance
    @State private var query = ""
    @State private var showDownloads = false
    @State private var confirmClear = false
    private var background: Color { appearance == .dark ? DesignTokens.Dark.background : DesignTokens.Light.background }
    private var surface: Color { appearance == .dark ? DesignTokens.Dark.surface : DesignTokens.Light.surface }
    private var secondary: Color { appearance == .dark ? DesignTokens.Dark.secondary : DesignTokens.Light.secondary }
    private var foreground: Color { appearance == .dark ? DesignTokens.Dark.text : DesignTokens.Light.text }
    private var accent: Color { appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent }
    private var running: CatalogCategory? { if case .downloading(let category, _, _) = archive.snapshot.download { return category }; return nil }
    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 18) {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("YOUR PERSONAL HOLOCRON").font(.caption).tracking(2).foregroundStyle(DesignTokens.Hero.accent)
                        Text("The galaxy.\nTo go.").font(.largeTitle.weight(.medium)).accessibilityAddTraits(.isHeader)
                        Text("Saved stories, wherever you travel.").font(.subheadline).foregroundStyle(DesignTokens.Hero.secondary)
                    }.padding(24).padding(.top, 24).frame(maxWidth: .infinity, alignment: .leading).foregroundStyle(DesignTokens.Hero.text)
                        .background { GeometryReader { geometry in
                            Image("hero_starships").resizable().scaledToFill().frame(width: geometry.size.width, height: geometry.size.height).clipped().accessibilityHidden(true)
                                .overlay(LinearGradient(colors: [DesignTokens.Hero.background.opacity(0.5), DesignTokens.Hero.background], startPoint: .top, endPoint: .bottom))
                        } }
                    VStack(alignment: .leading, spacing: 18) {
                        Toggle(isOn: Binding(get: { archive.snapshot.offline }, set: { value in archive.offline(value) })) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Offline mode").font(.headline)
                                Text("Browse saved records without a connection").font(.caption).foregroundStyle(secondary)
                            }
                        }.accessibilityIdentifier("offline-mode")
                        DisclosureGroup("Download collections", isExpanded: $showDownloads) {
                            VStack(alignment: .leading, spacing: 12) {
                                ForEach(archive.snapshot.collections, id: \.category) { collection in downloadRow(collection) }
                                Button("Clear saved cache", role: .destructive) { confirmClear = true }.buttonStyle(.bordered)
                            }.padding(.top, 12)
                        }
                        downloadNotice
                        if archive.snapshot.error != nil { Text("The archive could not save this change. Check available storage and try again.").font(.caption) }
                        TextField("Search saved records", text: $query, prompt: Text("Search saved records").foregroundStyle(secondary), axis: .vertical)
                            .lineLimit(1...3).textFieldStyle(.plain).padding(14)
                            .background(surface, in: RoundedRectangle(cornerRadius: DesignTokens.Shape.control))
                            .textInputAutocapitalization(.never).autocorrectionDisabled().accessibilityIdentifier("saved-search")
                            .onChange(of: query) { _, value in archive.search(value) }
                        Picker("Saved records", selection: Binding(get: { archive.snapshot.favoritesOnly }, set: { value in archive.favoritesOnly(value) })) {
                            Text("All saved").tag(false)
                            Text("Favorites").tag(true)
                        }.pickerStyle(.segmented)
                        Text("\(archive.snapshot.entries.count) saved \(archive.snapshot.entries.count == 1 ? "record" : "records")").font(.title2).accessibilityAddTraits(.isHeader)
                        Text("Search covers saved records only. Download a collection to keep every story.").font(.caption).foregroundStyle(secondary)
                        if archive.snapshot.entries.isEmpty { Text("No saved signals yet. Explore a collection or download it for your next journey.").foregroundStyle(secondary) }
                        ForEach(archive.snapshot.entries, id: \.identity) { saved in
                            NavigationLink {
                                if saved.category == .people { CharacterDestination(id: saved.entry.id) }
                                else { CatalogDestination(category: saved.category, id: saved.entry.id) }
                            } label: {
                                HStack(spacing: 14) {
                                    Image(saved.category.artwork).resizable().scaledToFill().frame(width: 64, height: 64).clipShape(RoundedRectangle(cornerRadius: 16)).accessibilityHidden(true)
                                    VStack(alignment: .leading, spacing: 6) {
                                        Text(saved.entry.name).font(.headline).foregroundStyle(foreground)
                                        Text(saved.category.title + (saved.favorite ? " · Favorite" : " · Saved")).font(.caption).foregroundStyle(accent)
                                    }.frame(maxWidth: .infinity, alignment: .leading)
                                }.padding(14).background(surface, in: RoundedRectangle(cornerRadius: 24))
                            }.buttonStyle(.plain).accessibilityIdentifier("saved-\(saved.identity)")
                        }
                    }.padding(.horizontal, 24).padding(.bottom, 32)
                }
            }.background(background).foregroundStyle(foreground).navigationTitle("Saved archive").navigationBarTitleDisplayMode(.inline)
                .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
                .onAppear { query = archive.snapshot.query }
                .alert("Clear saved cache?", isPresented: $confirmClear) {
                    Button("Clear cache", role: .destructive) { archive.clearCache() }
                    Button("Cancel", role: .cancel) {}
                } message: { Text("Favorites and their records stay on this device. Other saved records and downloaded collections will be removed.") }
        }.tint(accent)
    }
    private func downloadRow(_ collection: SavedCollection) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(collection.category.title).font(.headline)
            Text(collection.complete ? "Complete · \(collection.savedCount) \(collection.savedCount == 1 ? "record" : "records")" : "\(collection.savedCount) saved · \(collection.totalCount.map(String.init) ?? "?") total · Partial").font(.caption).foregroundStyle(secondary)
            if case .downloading(let category, let completed, let total) = archive.snapshot.download, category == collection.category {
                ProgressView().accessibilityLabel("Downloading \(completed) of \(total) records")
                Text("Downloading \(completed) of \(total) records").font(.caption)
                Button("Cancel download") { archive.cancelDownload() }.buttonStyle(.bordered)
            } else {
                Button((collection.complete ? "Update " : "Download ") + collection.category.title.lowercased()) { archive.download(collection.category) }
                    .buttonStyle(.bordered).disabled(archive.snapshot.offline || running != nil)
            }
        }.frame(maxWidth: .infinity, alignment: .leading).padding(16).background(surface, in: RoundedRectangle(cornerRadius: 20))
    }
    @ViewBuilder private var downloadNotice: some View {
        switch archive.snapshot.download {
        case .ready(let category, let count): Text("\(category.title): \(count) \(count == 1 ? "record" : "records") ready offline").font(.caption).foregroundStyle(accent)
        case .failed(let category, _):
            Text("Download interrupted. Saved records are still available.").font(.caption)
            Button("Retry download") { archive.download(category) }.disabled(archive.snapshot.offline)
        default: EmptyView()
        }
    }
}
private extension SavedEntry { var identity: String { "\(category.title.lowercased())-\(entry.id)" } }
