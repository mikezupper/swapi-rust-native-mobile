import SwiftUI
import WidgetKit

struct DiscoveryEntry: TimelineEntry { let date: Date; let cards: [WidgetCard] }
struct DiscoveryProvider: TimelineProvider {
    func placeholder(in context: Context) -> DiscoveryEntry { DiscoveryEntry(date: .now, cards: []) }
    func getSnapshot(in context: Context, completion: @escaping (DiscoveryEntry) -> Void) { completion(DiscoveryEntry(date: .now, cards: WidgetStorage.read())) }
    func getTimeline(in context: Context, completion: @escaping (Timeline<DiscoveryEntry>) -> Void) {
        completion(Timeline(entries: [DiscoveryEntry(date: .now, cards: WidgetStorage.read())], policy: .never))
    }
}
struct DiscoveryWidgetView: View {
    let entry: DiscoveryEntry
    @Environment(\.widgetFamily) private var family
    var body: some View {
        WidgetContentView(cards: entry.cards, medium: family == .systemMedium)
            .containerBackground(Color(red: 0.06, green: 0.09, blue: 0.1), for: .widget)
            .widgetURL(entry.cards.first.flatMap { URL(string: $0.url) })
    }
}
@main struct HolocronDiscoveryWidget: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "HolocronDiscovery", provider: DiscoveryProvider()) { entry in DiscoveryWidgetView(entry: entry) }
            .configurationDisplayName("Your galaxy")
            .description("A favorite and a discovery from your saved Holocron archive.")
            .supportedFamilies([.systemSmall, .systemMedium])
    }
}
