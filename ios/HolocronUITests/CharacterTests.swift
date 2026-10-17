import XCTest

final class CharacterTests: XCTestCase {
    @MainActor
    func testOfflineArchiveSurvivesRelaunch() async throws {
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/reset?scenario=people")!)
        let app = XCUIApplication()
        app.launchArguments = ["--endpoint", "http://127.0.0.1:18765/api/", "--archive", "--archive-reset"]
        app.launch()
        XCTAssertTrue(app.buttons["person-1"].waitForExistence(timeout: 15))
        if !app.buttons["person-1"].isHittable { app.swipeUp() }
        app.buttons["person-1"].tap()
        let favorite = app.buttons["favorite-record"]
        XCTAssertTrue(favorite.waitForExistence(timeout: 10))
        if !favorite.isHittable { app.swipeUp() }
        favorite.tap()
        XCTAssertTrue(app.buttons["Remove favorite"].waitForExistence(timeout: 5))
        app.navigationBars.buttons.element(boundBy: 0).tap()
        if !app.buttons["open-saved"].isHittable { app.swipeDown() }
        app.buttons["open-saved"].tap()
        XCTAssertTrue(app.switches["offline-mode"].waitForExistence(timeout: 5))
        app.buttons["Download collections"].tap()
        let planets = app.buttons["Download planets"]
        for _ in 0..<4 { if planets.isHittable { break }; app.swipeUp() }
        planets.tap()
        XCTAssertTrue(app.staticTexts["Complete · 1 records"].waitForExistence(timeout: 15))
        for _ in 0..<5 { if app.buttons["Download collections"].isHittable { break }; app.swipeDown() }
        app.buttons["Download collections"].tap()
        app.switches["offline-mode"].tap()
        app.segmentedControls.buttons["Favorites"].tap()
        capture(app, "Offline favorites")
        app.terminate()
        app.launchArguments.removeAll { $0 == "--archive-reset" }
        app.launch()
        XCTAssertTrue(app.buttons["open-saved"].waitForExistence(timeout: 15))
        app.buttons["open-saved"].tap()
        XCTAssertTrue(app.switches["offline-mode"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.switches["offline-mode"].value as? String, "1")
        app.segmentedControls.buttons["Favorites"].tap()
        let luke = app.buttons["saved-people-1"]
        XCTAssertTrue(luke.waitForExistence(timeout: 5))
        if !luke.isHittable { app.swipeUp() }
        luke.tap()
        XCTAssertTrue(app.buttons["Remove favorite"].waitForExistence(timeout: 10))
        capture(app, "Offline favorite after relaunch")
    }

    @MainActor
    func testEveryCatalogCategoryAndDetail() async throws {
        _ = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:18765/reset?scenario=people")!)
        let app = XCUIApplication()
        app.launchArguments = ["--endpoint", "http://127.0.0.1:18765/api/"]
        app.launch()
        XCTAssertTrue(app.buttons["category-planets"].waitForExistence(timeout: 15))
        let cases = [("planets", "1", "Tatooine", "Climate"), ("starships", "9", "Death Star", "Model"),
                     ("vehicles", "4", "Sand Crawler", "Model"), ("species", "3", "Wookiee", "Classification"),
                     ("films", "1", "A New Hope", "Episode")]
        for (category, id, name, fact) in cases {
            let picker = app.buttons["category-\(category)"]
            if !picker.isHittable { app.swipeDown() }
            picker.tap()
            let record = app.buttons["record-\(id)"]
            XCTAssertTrue(record.waitForExistence(timeout: 15))
            if !record.isHittable { app.swipeUp() }
            if category == "planets" { try app.performAccessibilityAudit() }
            capture(app, "Catalog \(category)")
            record.tap()
            XCTAssertTrue(app.staticTexts[name].waitForExistence(timeout: 15))
            XCTAssertTrue(app.staticTexts[fact].exists)
            if category == "planets" { try app.performAccessibilityAudit() }
            capture(app, "Catalog detail \(category)")
            app.navigationBars.buttons.element(boundBy: 0).tap()
        }
        app.buttons["category-people"].tap()
        XCTAssertTrue(app.buttons["person-1"].waitForExistence(timeout: 15))
    }

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
