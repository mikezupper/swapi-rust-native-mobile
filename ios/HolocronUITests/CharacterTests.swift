import XCTest

final class CharacterTests: XCTestCase {
    @MainActor
    func testAccessibilityAndBackgroundRestoration() async throws {
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/reset?scenario=people")!)
        let app = XCUIApplication()
        app.launchArguments = ["--endpoint", "http://127.0.0.1:18765/api/"]
        app.launch()
        XCTAssertTrue(app.buttons["person-1"].waitForExistence(timeout: 15))
        try app.performAccessibilityAudit()
        let search = app.textFields["people-search"]
        search.tap(); search.typeText("Leia")
        app.keyboards.buttons["Search"].tap()
        XCTAssertTrue(app.buttons["person-5"].waitForExistence(timeout: 15))
        XCUIDevice.shared.press(.home)
        app.activate()
        XCTAssertEqual(search.value as? String, "Leia")
        app.buttons["person-5"].tap()
        XCTAssertTrue(app.staticTexts["Leia Organa"].waitForExistence(timeout: 15))
        try app.performAccessibilityAudit()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertEqual(search.value as? String, "Leia")
        capture(app, "Restored query after background and detail")
    }

    @MainActor
    func testWarmScrollPerformance() async throws {
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/reset?scenario=performance")!)
        let app = XCUIApplication()
        app.launchArguments = ["--endpoint", "http://127.0.0.1:18765/api/"]
        app.launch()
        XCTAssertTrue(app.buttons["person-1"].waitForExistence(timeout: 15))
        print("Simulator reported maximum refresh rate: \(UIScreen.main.maximumFramesPerSecond) Hz")
        app.swipeUp(); app.swipeDown()
        let options = XCTMeasureOptions()
        options.iterationCount = 3
        measure(metrics: [XCTOSSignpostMetric.scrollingAndDecelerationMetric, XCTClockMetric()], options: options) {
            app.swipeUp(); app.swipeUp(); app.swipeDown(); app.swipeDown()
        }
        capture(app, "Measured fixture scrolling")
    }
    @MainActor
    func testRustFailureRetryAndCharacterFacts() async throws {
        let app = XCUIApplication()
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/reset?scenario=detail")!)
        app.launchArguments = ["--endpoint", "http://127.0.0.1:18765/api/", "--detail"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Reading the stars"].waitForExistence(timeout: 10))
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/release")!)
        let retry = app.buttons["Try again"]
        XCTAssertTrue(retry.waitForExistence(timeout: 15))
        if !retry.isHittable { app.swipeUp() }
        retry.tap()
        XCTAssertTrue(app.staticTexts["Luke Skywalker"].waitForExistence(timeout: 15))
        app.swipeUp()
        XCTAssertTrue(app.staticTexts["172 cm"].exists)
        XCTAssertTrue(app.staticTexts["4 films"].exists)
        let image = XCTAttachment(screenshot: app.screenshot())
        image.name = "Character from Rust"
        image.lifetime = .keepAlways
        add(image)
    }
    @MainActor
    func testPeopleNavigationPagingRetrySearchAndRefresh() async throws {
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/reset?scenario=people")!)
        let app = XCUIApplication()
        app.launchArguments = ["--endpoint", "http://127.0.0.1:18765/api/"]
        app.launch()
        XCTAssertTrue(app.buttons["person-2"].waitForExistence(timeout: 15))
        capture(app, "People from Rust")
        let droid = app.buttons["person-2"]
        if !droid.isHittable { app.swipeUp() }
        droid.tap()
        XCTAssertTrue(app.staticTexts["C-3PO"].waitForExistence(timeout: 15))
        capture(app, "Selected C-3PO detail")
        app.swipeUp()
        app.buttons["Refresh record"].tap()
        XCTAssertTrue(app.staticTexts["167 cm"].waitForExistence(timeout: 15))
        app.navigationBars.buttons.element(boundBy: 0).tap()
        let more = app.buttons["Load more people"]
        if !more.isHittable { app.swipeUp() }
        more.tap()
        let retry = app.buttons["Try again"]
        XCTAssertTrue(retry.waitForExistence(timeout: 15))
        if !retry.isHittable { app.swipeUp() }
        capture(app, "Page failure retains people")
        retry.tap()
        XCTAssertTrue(app.buttons["person-5"].waitForExistence(timeout: 15))
        app.swipeDown()
        let search = app.textFields["people-search"]
        if !search.isHittable { app.swipeDown() }
        search.tap()
        search.typeText("zzzz")
        XCTAssertTrue(app.staticTexts["No signals found"].waitForExistence(timeout: 15))
        capture(app, "Empty search")
        app.buttons["Clear search"].tap()
        search.tap()
        search.typeText("Leia")
        app.keyboards.buttons["Search"].tap()
        XCTAssertTrue(app.buttons["person-5"].waitForExistence(timeout: 15))
        XCTAssertFalse(app.buttons["person-1"].exists)
        app.buttons["Refresh people"].tap()
        XCTAssertTrue(app.buttons["person-5"].waitForExistence(timeout: 15))
        capture(app, "Search result from Rust")
    }
    @MainActor
    private func capture(_ app: XCUIApplication, _ name: String) {
        let image = XCTAttachment(screenshot: app.screenshot())
        image.name = name; image.lifetime = .keepAlways; add(image)
    }
}
