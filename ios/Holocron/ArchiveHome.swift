import SwiftUI

extension CatalogCategory {
    static let all: [Self] = [.people, .planets, .starships, .vehicles, .species, .films]
    // Atmospheric category art, not a depiction of an individual SWAPI record.
    var artwork: String {
        switch self {
        case .people: return "hero_people"
        case .planets: return "desert_orbit"
        case .starships: return "hero_starships"
        case .vehicles: return "hero_vehicles"
        case .species: return "hero_species"
        case .films: return "hero_films"
        }
    }
    var title: String {
        switch self {
        case .people: return "People"
        case .planets: return "Planets"
        case .starships: return "Starships"
        case .vehicles: return "Vehicles"
        case .species: return "Species"
        case .films: return "Films"
        }
    }
    var headline: String {
        switch self {
        case .people: return "A galaxy of\nstories."
        case .planets: return "Worlds beyond\nthe horizon."
        case .starships: return "Built for\nthe stars."
        case .vehicles: return "Adventure\nin motion."
        case .species: return "Life across\nthe galaxy."
        case .films: return "The stories\nthat started it."
        }
    }
    var subtitle: String {
        switch self {
        case .people: return "Discover the people behind the legends."
        case .planets: return "Desert suns. Ocean worlds. Distant homes."
        case .starships: return "Explore the vessels that cross the galaxy."
        case .vehicles: return "From desert crawlers to soaring speeders."
        case .species: return "Discover the galaxy’s many forms of life."
        case .films: return "Explore the saga, one episode at a time."
        }
    }
}

struct ArchiveHome: View {
    @ObservedObject var people: PeopleModel
    @State private var selected: CatalogCategory = .people
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    var body: some View {
        Group { if selected == .people {
            PeopleScreen(model: people, onCategory: { selected = $0 })
        } else {
            CatalogCollection(category: selected, onCategory: { selected = $0 }).id(selected)
        } }.animation(reduceMotion ? nil : .easeInOut(duration: 0.25), value: selected)
    }
}

struct CatalogCollection: View {
    @StateObject private var model: CatalogModel
    let category: CatalogCategory
    let onCategory: (CatalogCategory) -> Void
    init(category: CatalogCategory, onCategory: @escaping (CatalogCategory) -> Void) {
        self.category = category; self.onCategory = onCategory
        _model = StateObject(wrappedValue: CatalogModel(category: category))
    }
    var body: some View { CatalogScreen(model: model, onCategory: onCategory, category: category) }
}

struct ArchiveMonogram: View {
    let name: String
    @Environment(\.colorScheme) private var appearance
    private var background: Color { appearance == .dark ? DesignTokens.Dark.background : DesignTokens.Light.background }
    private var foreground: Color { appearance == .dark ? DesignTokens.Dark.text : DesignTokens.Light.text }
    var body: some View {
        // Keep the text's intrinsic bounds separate from the decorated tile.
        ZStack {
            RoundedRectangle(cornerRadius: 16).fill(background)
            Text(name.split(separator: " ").prefix(2).compactMap(\.first).map(String.init).joined())
                .font(.headline.weight(.semibold)).foregroundStyle(foreground)
                .fixedSize().background(background).accessibilityHidden(true)
        }.frame(width: 48, height: 48).accessibilityHidden(true)
    }
}

struct CategoryPicker: View {
    @Environment(\.dynamicTypeSize) private var typeSize
    let selected: CatalogCategory
    let onSelect: (CatalogCategory) -> Void
    @Environment(\.colorScheme) private var appearance
    private var accent: Color { appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent }
    private var onAccent: Color { appearance == .dark ? DesignTokens.Dark.onAccent : DesignTokens.Light.onAccent }
    private var surface: Color { appearance == .dark ? DesignTokens.Dark.surface : DesignTokens.Light.surface }
    var body: some View {
        let columns = typeSize.isAccessibilitySize ? 1 : (typeSize > .large ? 2 : 3)
        CategoryGridLayout(columns: columns) {
            ForEach(CatalogCategory.all, id: \.self) { category in
                Button { if selected != category { onSelect(category) } } label: {
                    Text(category.title).font(.subheadline.weight(.medium))
                        .fixedSize(horizontal: false, vertical: true).frame(maxWidth: .infinity, minHeight: 44)
                        .padding(.horizontal, 4)
                        .foregroundStyle(selected == category ? onAccent : accent)
                        .background(selected == category ? accent : surface, in: RoundedRectangle(cornerRadius: 14))
                }.buttonStyle(.plain)
                    .accessibilityAddTraits(selected == category ? .isSelected : [])
                    .accessibilityIdentifier("category-\(category.title.lowercased())")
            }
        }.padding(.horizontal, DesignTokens.Space.lg)
    }
}

// All six controls stay in the hierarchy as Dynamic Type changes their columns.
// A lazy grid can stop updating the buttons pushed beyond the visible viewport.
private struct CategoryGridLayout: Layout {
    let columns: Int
    private let spacing: CGFloat = 8

    private func cellWidth(_ width: CGFloat) -> CGFloat {
        max(0, (width - CGFloat(columns - 1) * spacing) / CGFloat(columns))
    }
    private func rowHeights(width: CGFloat, subviews: Subviews) -> [CGFloat] {
        let proposal = ProposedViewSize(width: cellWidth(width), height: nil)
        return stride(from: 0, to: subviews.count, by: columns).map { start in
            (start..<min(start + columns, subviews.count))
                .map { subviews[$0].sizeThatFits(proposal).height }.max() ?? 0
        }
    }
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout Void) -> CGSize {
        let ideal = (subviews.map { $0.sizeThatFits(.unspecified).width }.max() ?? 0)
            * CGFloat(columns) + CGFloat(columns - 1) * spacing
        let width = proposal.width.flatMap { $0.isFinite ? $0 : nil } ?? ideal
        let heights = rowHeights(width: width, subviews: subviews)
        return CGSize(width: width, height: heights.reduce(0, +) + CGFloat(max(0, heights.count - 1)) * spacing)
    }
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout Void) {
        let width = cellWidth(bounds.width)
        var y = bounds.minY
        for (row, height) in rowHeights(width: bounds.width, subviews: subviews).enumerated() {
            for column in 0..<columns {
                let index = row * columns + column
                guard index < subviews.count else { break }
                subviews[index].place(at: CGPoint(x: bounds.minX + CGFloat(column) * (width + spacing), y: y),
                                      anchor: .topLeading, proposal: ProposedViewSize(width: width, height: height))
            }
            y += height + spacing
        }
    }
}

struct CatalogDestination: View {
    @StateObject private var model: CatalogModel
    let category: CatalogCategory
    init(category: CatalogCategory, id: UInt32) {
        self.category = category
        _model = StateObject(wrappedValue: CatalogModel(category: category, recordId: id))
    }
    var body: some View { CatalogDetailScreen(model: model, category: category).toolbar(.visible, for: .navigationBar) }
}

struct CatalogDetailScreen: View {
    @ObservedObject var model: CatalogModel
    let category: CatalogCategory
    @Environment(\.colorScheme) private var appearance
    @ScaledMetric(relativeTo: .largeTitle) private var heroSize = DesignTokens.Typography.hero
    private var background: Color { appearance == .dark ? DesignTokens.Dark.background : DesignTokens.Light.background }
    private var surface: Color { appearance == .dark ? DesignTokens.Dark.surface : DesignTokens.Light.surface }
    private var foreground: Color { appearance == .dark ? DesignTokens.Dark.text : DesignTokens.Light.text }
    private var secondary: Color { appearance == .dark ? DesignTokens.Dark.secondary : DesignTokens.Light.secondary }
    private var accent: Color { appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent }
    private var record: CatalogEntry? { model.snapshot.items.first }
    private var busy: Bool { model.snapshot.phase == .loading || model.snapshot.phase == .refreshing }
    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 12) {
                    Spacer(minLength: 64)
                    Text(category.title.uppercased() + (record.map { String(format: " / %03d", $0.id) } ?? ""))
                        .font(.caption).tracking(2).foregroundStyle(DesignTokens.Hero.accent)
                    Text(record?.name ?? "Reading the stars").font(.system(size: heroSize, weight: .medium))
                        .fixedSize(horizontal: false, vertical: true).accessibilityAddTraits(.isHeader)
                }.padding(24).frame(maxWidth: .infinity, alignment: .leading).foregroundStyle(DesignTokens.Hero.text)
                    .background {
                        GeometryReader { geometry in
                            Image(category.artwork).resizable().scaledToFill().frame(width: geometry.size.width, height: geometry.size.height).clipped().accessibilityHidden(true)
                                .overlay(LinearGradient(colors: [DesignTokens.Hero.background.opacity(0.5), DesignTokens.Hero.background], startPoint: .top, endPoint: .bottom))
                        }
                    }
                VStack(alignment: .leading, spacing: 22) {
                    if busy { ProgressView().accessibilityLabel("Loading record") }
                    if case .failed(_, let retryable) = model.snapshot.phase {
                        Text("Signal interrupted").font(.title2)
                        Text("The archive could not load this record. Please try again.").foregroundStyle(secondary)
                        if retryable { Button("Try again", action: model.retry).buttonStyle(.borderedProminent) }
                    }
                    if let record {
                        CacheNotice(cache: model.snapshot.cache)
                        FavoriteButton(category: category, id: record.id)
                        Text("The details").font(.title2).accessibilityAddTraits(.isHeader)
                        VStack(alignment: .leading, spacing: 20) {
                            ForEach(record.facts, id: \.label) { fact in
                                VStack(alignment: .leading, spacing: 6) {
                                    Text(fact.label).font(.caption).foregroundStyle(secondary)
                                    Text(fact.value).font(.headline.weight(.medium)).fixedSize(horizontal: false, vertical: true)
                                }.frame(maxWidth: .infinity, alignment: .leading)
                            }
                        }.padding(20).background(surface, in: RoundedRectangle(cornerRadius: 24))
                        if let crawl = record.description {
                            Text("Opening crawl").font(.title2).accessibilityAddTraits(.isHeader)
                            Text(crawl).font(.body).lineSpacing(6).foregroundStyle(secondary).textSelection(.enabled)
                        }
                        Text("Record from the Star Wars API.").font(.caption).foregroundStyle(secondary)
                    }
                    Button("Refresh record", action: model.refresh).buttonStyle(.bordered).disabled(busy)
                }.padding(24).frame(maxWidth: .infinity, alignment: .leading)
            }
        }.background(background).foregroundStyle(foreground).tint(accent).navigationTitle(category.title).navigationBarTitleDisplayMode(.inline)
    }
}
