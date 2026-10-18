import Foundation

struct WidgetCard: Codable, Sendable {
    let name: String
    let subtitle: String
    let url: String
    let artwork: String
}
enum WidgetStorage {
    static var folder: URL? {
        guard let group = Bundle.main.object(forInfoDictionaryKey: "HolocronAppGroup") as? String else { return nil }
        return FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)
    }
    static func read() -> [WidgetCard] {
        guard let file = folder?.appendingPathComponent("discovery.json"), let data = try? Data(contentsOf: file), let cards = try? JSONDecoder().decode([WidgetCard].self, from: data) else { return [] }
        return cards
    }
}
