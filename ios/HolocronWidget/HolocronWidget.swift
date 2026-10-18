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
        VStack(alignment: .leading, spacing: 12) {
            Text("HOLOCRON  ✦").font(.caption2.weight(.semibold)).tracking(2).foregroundStyle(Color(red: 0.61, green: 0.92, blue: 0.86))
            if entry.cards.isEmpty {
                Text("Your galaxy, closer.").font(.title3.weight(.semibold))
                Text("Open Holocron to save a discovery.").font(.caption)
            } else if family == .systemMedium {
                HStack(alignment: .top, spacing: 20) { ForEach(Array(entry.cards.prefix(2).enumerated()), id: \.offset) { _, card in cardView(card) } }
            } else if let card = entry.cards.first { cardView(card) }
        }.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .foregroundStyle(Color(red: 0.95, green: 0.95, blue: 0.92))
            .containerBackground(for: .widget) {
                ZStack {
                    Color(red: 0.06, green: 0.09, blue: 0.1)
                    if let card = entry.cards.last { Image(card.artwork).resizable().scaledToFill().opacity(0.25) }
                    LinearGradient(colors: [.clear, Color(red: 0.06, green: 0.09, blue: 0.1)], startPoint: .top, endPoint: .bottom)
                }
            }
            .widgetURL(entry.cards.first.flatMap { URL(string: $0.url) })
    }
    private func cardView(_ card: WidgetCard) -> some View {
        Link(destination: URL(string: card.url)!) {
            VStack(alignment: .leading, spacing: 8) {
                Text(card.name).font(.headline).lineLimit(3)
                Text(card.subtitle).font(.caption2).lineLimit(2)
                Text("Explore →").font(.caption).foregroundStyle(Color(red: 0.91, green: 0.73, blue: 0.53))
            }.frame(maxWidth: .infinity, alignment: .leading)
        }
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
