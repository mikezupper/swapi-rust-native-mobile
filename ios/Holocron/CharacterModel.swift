import Combine
import Foundation

@MainActor
final class CharacterModel: ObservableObject {
    @Published private(set) var snapshot = CharacterSnapshot(revision: 0, state: .idle)
    private var session: CharacterSession?
    private var subscription: Subscription?
    private var stopped = false
    private let personId: UInt32

    init(core injectedCore: CharacterSession? = nil, autoload: Bool = true, personId: UInt32 = 1) {
        self.personId = personId
        do {
            var core: CharacterSession
            if let injectedCore { core = injectedCore } else {
            #if DEBUG
            let args = ProcessInfo.processInfo.arguments
            if let index = args.firstIndex(of: "--endpoint"), args.count > index + 1 {
                core = try CharacterSession.withEndpoint(baseUrl: args[index + 1])
            } else { core = try CharacterSession() }
            #else
            core = try CharacterSession()
            #endif
            }
            session = core
            subscription = try core.subscribe(listener: StateListener(owner: self))
            if autoload { try core.load(personId: personId) }
        } catch {
            snapshot = CharacterSnapshot(revision: 1, state: .failed(personId: personId, kind: .network, retryable: false))
        }
    }
    func receive(_ value: CharacterSnapshot) {
        guard !stopped, value.revision > snapshot.revision else { return }
        snapshot = value
    }
    func retry() { guard !stopped else { return }; try? session?.retry() }
    func refresh() { guard !stopped else { return }; try? session?.load(personId: personId) }
    func stop() {
        stopped = true
        subscription?.cancel()
        subscription = nil
        session?.shutdown()
        session = nil
    }
    deinit { subscription?.cancel(); session?.shutdown() }
}

@MainActor
private final class StateListener: CharacterListener {
    private weak var owner: CharacterModel?
    init(owner: CharacterModel) { self.owner = owner }
    nonisolated func onState(snapshot: CharacterSnapshot) throws {
        Task { @MainActor [weak self] in self?.owner?.receive(snapshot) }
    }
}
