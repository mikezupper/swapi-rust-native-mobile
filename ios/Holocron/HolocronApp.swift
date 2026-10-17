import SwiftUI

@main
struct HolocronApp: App {
    @StateObject private var model: PeopleModel
    init() {
        #if DEBUG
        // The unit-test host must not issue production requests at app startup.
        let isUnitTestHost = NSClassFromString("XCTestCase") != nil || ProcessInfo.processInfo.arguments.contains("--detail")
        _model = StateObject(wrappedValue: PeopleModel(autoload: !isUnitTestHost))
        #else
        _model = StateObject(wrappedValue: PeopleModel())
        #endif
    }
    var body: some Scene {
        WindowGroup {
            #if DEBUG
            if ProcessInfo.processInfo.arguments.contains("--detail") {
                CharacterDestination(id: 1)
            } else { ArchiveHome(people: model) }
            #else
            ArchiveHome(people: model)
            #endif
        }
    }
}

struct CharacterScreen: View {
    @ObservedObject var model: CharacterModel
    @Environment(\.colorScheme) private var appearance
    @ScaledMetric(relativeTo: .largeTitle) private var heroSize = DesignTokens.Typography.hero
    @Environment(\.dynamicTypeSize) private var typeSize
    private var background: Color { appearance == .dark ? DesignTokens.Hero.background : DesignTokens.Light.background }
    private var foreground: Color { appearance == .dark ? DesignTokens.Hero.text : DesignTokens.Light.text }
    private var secondary: Color { appearance == .dark ? DesignTokens.Dark.secondary : DesignTokens.Light.secondary }
    private var surface: Color { appearance == .dark ? DesignTokens.Dark.surface : DesignTokens.Light.surface }
    private var accent: Color { appearance == .dark ? DesignTokens.Dark.accent : DesignTokens.Light.accent }
    private var character: Character? {
        if case .ready(let person) = model.snapshot.state { return person }
        return nil
    }
    private var title: String {
        if let character { return character.name }
        if case .failed = model.snapshot.state { return "Signal interrupted" }
        return "Reading the stars"
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                hero
                VStack(alignment: .leading, spacing: 26) {
                    switch model.snapshot.state {
                    case .ready(let person): details(person)
                    case .failed(_, let kind, let retryable):
                        Text(message(kind)).font(.title3)
                        if retryable {
                            Button("Try again", action: model.retry).buttonStyle(.borderedProminent)
                        }
                    case .loading:
                        ProgressView().tint(accent).accessibilityLabel("Loading record")
                        Text("Connecting to the galactic archive…").foregroundStyle(secondary)
                    case .idle:
                        Button("Open the archive", action: model.refresh).buttonStyle(.borderedProminent)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(DesignTokens.Space.lg)
            }
        }
        .background(background)
        .foregroundStyle(foreground)
        .tint(accent)
        .ignoresSafeArea(edges: .top)
    }

    private var hero: some View {
        VStack(alignment: .leading, spacing: 12) {
            Spacer(minLength: 216)
            Text("THE GALACTIC ARCHIVE" + (character.map { String(format: " / %03d", $0.id) } ?? ""))
                .font(.caption).tracking(1.6).foregroundStyle(DesignTokens.Hero.accent)
                .fixedSize(horizontal: false, vertical: true)
                .padding(8).background(DesignTokens.Hero.background, in: RoundedRectangle(cornerRadius: 8))
            Text(title).font(.system(size: heroSize, weight: .medium))
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityAddTraits(.isHeader)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text("One life. A galaxy of connections.").font(.subheadline)
                .foregroundStyle(DesignTokens.Hero.secondary)
        }
        .padding(.horizontal, DesignTokens.Space.lg).padding(.bottom, DesignTokens.Space.lg)
        .frame(maxWidth: .infinity, minHeight: 425, alignment: .leading)
        .background {
            GeometryReader { geometry in
                Image(CatalogCategory.people.artwork).resizable().scaledToFill().accessibilityHidden(true)
                    .frame(width: geometry.size.width, height: geometry.size.height).clipped()
                    .overlay(LinearGradient(colors: [DesignTokens.Hero.background.opacity(0.4), DesignTokens.Hero.background.opacity(0.25), DesignTokens.Hero.background], startPoint: .top, endPoint: .bottom))
            }
        }
        .foregroundStyle(DesignTokens.Hero.text)
        .clipShape(UnevenRoundedRectangle(bottomLeadingRadius: DesignTokens.Shape.hero, bottomTrailingRadius: DesignTokens.Shape.hero))
    }

    @ViewBuilder private func details(_ person: Character) -> some View {
        let layout = typeSize.isAccessibilitySize ? AnyLayout(VStackLayout(alignment: .leading, spacing: 20)) : AnyLayout(HStackLayout(alignment: .top, spacing: 16))
        layout {
            fact("Height", measurement(person.heightCm, unit: "cm"))
            fact("Mass", measurement(person.massKg, unit: "kg"))
            fact("Born", person.birthYear)
        }
        .padding(22).background(surface, in: RoundedRectangle(cornerRadius: DesignTokens.Shape.card))
        VStack(alignment: .leading, spacing: 0) {
            Text("The details").font(.title2.weight(.medium)).accessibilityAddTraits(.isHeader).padding(.bottom, 12)
            row("Eye color", person.eyeColor)
            row("Hair color", person.hairColor)
            row("Film appearances", "\(person.filmCount) films")
        }
        Label("Record from the Star Wars API.\nBBY: Before the Battle of Yavin.", systemImage: "info.circle")
            .font(.caption).foregroundStyle(secondary)
        Button("Refresh record", action: model.refresh).buttonStyle(.bordered)
    }
    private func fact(_ label: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(label).font(.caption).foregroundStyle(secondary)
            Text(value).font(.headline.weight(.medium)).fixedSize(horizontal: false, vertical: true)
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
    private func row(_ label: String, _ value: String) -> some View {
        let layout = typeSize.isAccessibilitySize ? AnyLayout(VStackLayout(alignment: .leading, spacing: 8)) : AnyLayout(HStackLayout(alignment: .top, spacing: 16))
        return VStack {
            layout {
                Text(label).foregroundStyle(secondary).fixedSize(horizontal: false, vertical: true).frame(maxWidth: .infinity, alignment: .leading)
                Text(value).fixedSize(horizontal: false, vertical: true).frame(maxWidth: .infinity, alignment: typeSize.isAccessibilitySize ? .leading : .trailing)
            }.font(.subheadline).padding(.vertical, 16)
            Divider()
        }
    }
    private func measurement(_ value: Measurement, unit: String) -> String {
        switch value {
        case .known(let number): return "\(number.formatted()) \(unit)"
        case .unknown: return "Unknown"
        case .notApplicable: return "N/A"
        }
    }
    private func message(_ kind: FailureKind) -> String {
        switch kind {
        case .network: return "The archive is out of reach. Check your connection and try again."
        case .timeout: return "The archive took too long to respond. Try again in a moment."
        case .http(let status): return status == 404 ? "This record could not be found." : "The archive is temporarily unavailable."
        case .invalidData: return "This record arrived in a format we could not read."
        case .invalidRequest: return "This record cannot be requested."
        }
    }
}
