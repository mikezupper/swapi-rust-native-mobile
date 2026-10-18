import Combine
import Foundation

@MainActor
final class CatalogModel: ObservableObject {
    @Published private(set) var snapshot = CatalogSnapshot(revision: 0, query: "", items: [], total: 0, hasMore: false, phase: .idle)
    private var session: CatalogSession?
    private var subscription: Subscription?
    private var stopped = false
    init(category: CatalogCategory, recordId: UInt32? = nil, relatedIds: [UInt32]? = nil, core injectedCore: CatalogSession? = nil, autoload: Bool = true) {
        do {
            let core: CatalogSession
            if let relatedIds {
                if let archive = ArchiveModel.shared.session { core = try archive.relatedSession(category: category, ids: relatedIds) }
                else {
                    let args = ProcessInfo.processInfo.arguments
                    #if DEBUG
                    let endpoint = args.firstIndex(of: "--endpoint").flatMap { args.count > $0 + 1 ? args[$0 + 1] : nil }
                    #else
                    let endpoint: String? = nil
                    #endif
                    core = try CatalogSession.relatedRecords(category: category, ids: relatedIds, baseUrl: endpoint)
                }
            } else if let injectedCore { core = injectedCore } else if let archive = ArchiveModel.shared.session { core = try archive.catalogSession(category: category, recordId: recordId) } else {
                #if DEBUG
                let args = ProcessInfo.processInfo.arguments
                if let index = args.firstIndex(of: "--endpoint"), args.count > index + 1 {
                    core = try CatalogSession.withEndpoint(baseUrl: args[index + 1], category: category, recordId: recordId)
                } else { core = try CatalogSession(category: category, recordId: recordId) }
                #else
                core = try CatalogSession(category: category, recordId: recordId)
                #endif
            }
            session = core
            subscription = try core.subscribe(listener: CatalogStateListener(owner: self))
            if autoload { try core.refresh() }
        } catch {
            snapshot = CatalogSnapshot(revision: 1, query: "", items: [], total: 0, hasMore: false, phase: .failed(kind: .network, retryable: false))
        }
    }
    func receive(_ value: CatalogSnapshot) {
        guard !stopped, value.revision > snapshot.revision else { return }
        snapshot = value
    }
    func search(_ query: String) { guard !stopped else { return }; try? session?.search(query: query) }
    func refresh() { guard !stopped else { return }; try? session?.refresh() }
    func loadMore() { guard !stopped else { return }; try? session?.loadMore() }
    func retry() { guard !stopped else { return }; try? session?.retry() }
    func stop() {
        stopped = true; subscription?.cancel(); subscription = nil
        session?.shutdown(); session = nil
    }
    deinit { subscription?.cancel(); session?.shutdown() }
}
@MainActor
private final class CatalogStateListener: CatalogListener {
    private weak var owner: CatalogModel?
    init(owner: CatalogModel) { self.owner = owner }
    nonisolated func onState(snapshot: CatalogSnapshot) throws {
        Task { @MainActor [weak self] in self?.owner?.receive(snapshot) }
    }
}
