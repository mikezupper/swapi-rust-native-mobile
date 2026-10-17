import Combine
import XCTest
@testable import Holocron

final class CharacterModelTests: XCTestCase {
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
