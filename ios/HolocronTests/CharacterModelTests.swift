import Combine
import XCTest
@testable import Holocron

final class CharacterModelTests: XCTestCase {
    func testWidgetAppGroupCanShareAnAtomicSnapshot() throws {
        let folder = try XCTUnwrap(WidgetStorage.folder, "The app and WidgetKit extension need the configured App Group")
        let file = folder.appendingPathComponent("verification-\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: file) }
        let cards = [WidgetCard(name: "Fixture world", subtitle: "Saved discovery · Planets", url: "holocron://record/planets/1", artwork: "desert_orbit")]
        try JSONEncoder().encode(cards).write(to: file, options: .atomic)
        let restored = try JSONDecoder().decode([WidgetCard].self, from: Data(contentsOf: file))
        XCTAssertEqual(restored.first?.name, "Fixture world")
        XCTAssertEqual(parseRecordLink(value: try XCTUnwrap(restored.first?.url))?.id, 1)
    }

    @MainActor
    func testCatalogAdapterRejectsStaleDeliveryAndReleasesSession() throws {
        let core = try CatalogSession(category: .planets, recordId: nil)
        var model: CatalogModel? = CatalogModel(category: .planets, core: core, autoload: false)
        weak var weakModel = model
        let current = CatalogSnapshot(revision: 7, query: "Tatooine", items: [], total: 0, hasMore: false, phase: .loading)
        model?.receive(current)
        model?.receive(CatalogSnapshot(revision: 6, query: "old", items: [], total: 0, hasMore: false, phase: .ready))
        XCTAssertEqual(model?.snapshot, current)
        model?.stop()
        model?.receive(CatalogSnapshot(revision: 8, query: "late", items: [], total: 0, hasMore: false, phase: .ready))
        XCTAssertEqual(model?.snapshot, current)
        model = nil
        XCTAssertNil(weakModel)
        XCTAssertThrowsError(try core.refresh())
    }

    @MainActor
    func testPeopleAdapterRejectsStaleDeliveryAndReleasesSession() throws {
        let core = try PeopleSession()
        var model: PeopleModel? = PeopleModel(core: core, autoload: false)
        weak var weakModel = model
        let current = PeopleSnapshot(revision: 7, query: "Leia", people: [], total: 0, hasMore: false, phase: .loading)
        model?.receive(current)
        model?.receive(PeopleSnapshot(revision: 6, query: "Luke", people: [], total: 0, hasMore: false, phase: .ready))
        XCTAssertEqual(model?.snapshot, current)
        model?.stop()
        model?.receive(PeopleSnapshot(revision: 8, query: "Luke", people: [], total: 0, hasMore: false, phase: .ready))
        XCTAssertEqual(model?.snapshot, current)
        model = nil
        XCTAssertNil(weakModel)
        XCTAssertThrowsError(try core.refresh())
    }
    @MainActor
    func testRustCallbacksArriveOnMainActorWithTypedFailure() async throws {
        let core = try CharacterSession()
        let model = CharacterModel(core: core, autoload: false)
        let received = expectation(description: "Rust invalid-request state")
        let observation = model.$snapshot.sink { value in
            XCTAssertTrue(Thread.isMainThread)
            if case .failed(let id, let kind, let retryable) = value.state {
                XCTAssertEqual(id, 0)
                XCTAssertEqual(kind, .invalidRequest)
                XCTAssertFalse(retryable)
                received.fulfill()
            }
        }
        defer { observation.cancel(); model.stop() }
        try core.load(personId: 0)
        await fulfillment(of: [received], timeout: 3)
    }

    @MainActor
    func testStaleAndAlreadyQueuedDeliveriesCannotReplaceCurrentState() throws {
        let model = CharacterModel(core: try CharacterSession(), autoload: false)
        model.receive(CharacterSnapshot(revision: 5, state: .loading(personId: 2)))
        model.receive(CharacterSnapshot(revision: 4, state: .idle))
        XCTAssertEqual(model.snapshot.revision, 5)
        model.stop()
        model.receive(CharacterSnapshot(revision: 6, state: .failed(personId: 2, kind: .network, retryable: true)))
        XCTAssertEqual(model.snapshot.state, .loading(personId: 2))
        model.stop()
    }

    @MainActor
    func testModelReleasesSubscriptionAndShutsDownRustWithoutRetainCycle() throws {
        let core = try CharacterSession()
        var model: CharacterModel? = CharacterModel(core: core, autoload: false)
        weak var weakModel = model
        model = nil
        XCTAssertNil(weakModel)
        XCTAssertThrowsError(try core.load(personId: 1)) { error in
            guard case BridgeError.Closed = error else { return XCTFail("Expected a closed session: \(error)") }
        }
    }
}
