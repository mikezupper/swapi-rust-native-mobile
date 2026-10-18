import Foundation
import WidgetKit

@MainActor enum WidgetSnapshotWriter {
    private static var previous: Data?
    private static let writer = DispatchQueue(label: "dev.holocron.widget-snapshot", qos: .utility)
    static func publish(_ entries: [SavedEntry]) {
        #if DEBUG
        // Deterministic fixture sessions never replace the user's home-screen data.
        if ProcessInfo.processInfo.arguments.contains("--endpoint") { return }
        #endif
        let cards = entries.map { saved in WidgetCard(name: saved.entry.name,
            subtitle: "\(saved.favorite ? "Favorite" : "Saved discovery") · \(saved.category.title)",
            url: recordLink(category: saved.category, id: saved.entry.id), artwork: saved.category.artwork) }
        guard let data = try? JSONEncoder().encode(cards), data != previous, let folder = WidgetStorage.folder else { return }
        previous = data
        writer.async {
            do {
                try data.write(to: folder.appendingPathComponent("discovery.json"), options: .atomic)
                WidgetCenter.shared.reloadTimelines(ofKind: "HolocronDiscovery")
            } catch {
                // Retry the same projection on the next archive update.
                Task { @MainActor in if previous == data { previous = nil } }
            }
        }
    }
}
