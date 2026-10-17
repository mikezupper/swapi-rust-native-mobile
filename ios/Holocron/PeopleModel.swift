import Combine
import Foundation

@MainActor
final class PeopleModel: ObservableObject {
    @Published private(set) var snapshot = PeopleSnapshot(revision: 0, query: "", people: [], total: 0, hasMore: false, phase: .idle)
    private var session: PeopleSession?
    private var subscription: Subscription?
    private var stopped = false
    init(core injectedCore: PeopleSession? = nil, autoload: Bool = true) {
        do {
            let core: PeopleSession
            if let injectedCore { core = injectedCore } else {
                #if DEBUG
                let args = ProcessInfo.processInfo.arguments
                if let index = args.firstIndex(of: "--endpoint"), args.count > index + 1 {
                    core = try PeopleSession.withEndpoint(baseUrl: args[index + 1])
                } else { core = try PeopleSession() }
                #else
                core = try PeopleSession()
                #endif
            }
            session = core
            subscription = try core.subscribe(listener: PeopleStateListener(owner: self))
            if autoload { try core.refresh() }
        } catch {
            snapshot = PeopleSnapshot(revision: 1, query: "", people: [], total: 0, hasMore: false, phase: .failed(kind: .network, retryable: false))
        }
    }
    func receive(_ value: PeopleSnapshot) {
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
private final class PeopleStateListener: PeopleListener {
    private weak var owner: PeopleModel?
    init(owner: PeopleModel) { self.owner = owner }
    nonisolated func onState(snapshot: PeopleSnapshot) throws {
        Task { @MainActor [weak self] in self?.owner?.receive(snapshot) }
    }
}
