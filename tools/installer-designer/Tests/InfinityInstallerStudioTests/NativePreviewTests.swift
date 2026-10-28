import AppKit
import XCTest
@testable import InfinityInstallerStudio

private actor ControlledNativeRenderer: NativePreviewRunning {
    private var requests: [Int: CheckedContinuation<NativePreviewFrame, any Error>] = [:]
    private var waiters: [Int: CheckedContinuation<Void, Never>] = [:]

    // ------------------------=
    // FUNC: render
    // DESC: Defers typed frames until the test explicitly releases them, including intentionally out-of-order completions.
    // ------------------=
    func render(_ snapshot: NativePreviewSnapshot) async throws -> NativePreviewFrame {
        try await withCheckedThrowingContinuation { continuation in
            requests[snapshot.screen] = continuation
            waiters.removeValue(forKey: snapshot.screen)?.resume()
        }
    }

    // ------------------------=
    // FUNC: wait
    // DESC: Waits deterministically for a render invocation without timer-based polling.
    // ------------------=
    func wait(for screen: Int) async {
        if requests[screen] != nil { return }
        await withCheckedContinuation { waiters[screen] = $0 }
    }

    // ------------------------=
    // FUNC: finish
    // DESC: Supplies either valid or malformed renderer bytes through the same asynchronous interface.
    // ------------------=
    func finish(_ screen: Int, value: UInt8, malformed: Bool = false) {
        requests.removeValue(forKey: screen)?.resume(returning: NativePreviewFrame(
            pixels: Data(repeating: value, count: malformed ? 4 : 320 * 240 * 4), width: 320, height: 240))
    }
}

final class NativePreviewTests: XCTestCase {
    // ------------------------=
    // FUNC: testSettingsRowFocusUsesNativeRange
    // DESC: Admits all bounded native row-focus identities and rejects values outside the shared control range.
    // ------------------=
    func testSettingsRowFocusUsesNativeRange() throws {
        var simulation = NativePreviewSimulation()
        for focus in 0...255 {
            simulation.controlFocus = focus
            XCTAssertNoThrow(try simulation.arguments())
        }
        for focus in [-1, 256, Int.max] {
            simulation.controlFocus = focus
            XCTAssertThrowsError(try simulation.arguments())
        }
    }

    // ------------------------=
    // FUNC: testSettingsKeyboardFocusReachesNativePainter
    // DESC: Sends Studio's row focus through the real renderer and verifies that it changes pixels without changing authored element geometry.
    // ------------------=
    @MainActor func testSettingsKeyboardFocusReachesNativePainter() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 4)
        let runner = NativePreviewRunner()
        store.nativePreviewSimulation.controlFocus = 0
        let normal = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        store.nativePreviewSimulation.controlFocus = 128
        let focused = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertEqual(normal.pixels.count, focused.pixels.count)
        XCTAssertNotEqual(normal.pixels, focused.pixels)
        XCTAssertEqual(normal.bounds, focused.bounds)
    }

    // ------------------------=
    // FUNC: testSettingsChildPixelsFollowAuthoredGeometryAndVisibility
    // DESC: Compares actual production frames for row descriptions and detail surfaces at two viewport sizes, including hidden/transparent parity.
    // ------------------=
    @MainActor func testSettingsChildPixelsFollowAuthoredGeometryAndVisibility() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 1)
        let runner = NativePreviewRunner()
        for role in [StudioElementRole.settingsSummaryDescription, .settingsDetailDescription, .settingsDetailSurface] {
            let child = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == role })
            store.selectElement(child.id)
            store.nativePreviewSimulation.expandedRow = 0
            store.nativePreviewSimulation.scroll = 0
            for (width, height) in [(1280, 800), (1920, 1080)] {
                let baseline = try await runner.render(NativePreviewSnapshot.capture(store, width: width, height: height))
                let before = try XCTUnwrap(baseline.bounds.first { $0.id == child.id })
                store.updateSelected {
                    $0.frame = role == .settingsDetailSurface
                        ? CanvasRect(x: 80, y: 80, width: 840, height: 820)
                        : CanvasRect(x: 160, y: 420, width: 400, height: 300)
                    $0.fill = StudioColor(red: 190, green: 105, blue: 240, alpha: 230)
                    if role == .settingsDetailSurface {
                        $0.surfaceMaterial = .matte; $0.surfaceCorners = .rounded; $0.cornerRadius = 12
                    } else {
                        $0.text = "Editable native description"; $0.fontSize = 20
                    }
                }
                let edited = try XCTUnwrap(store.selectedElement)
                let changed = try await runner.render(NativePreviewSnapshot.capture(store, width: width, height: height))
                let bounds = try XCTUnwrap(changed.bounds.first { $0.id == child.id })
                let parent = try XCTUnwrap(bounds.parentBounds)
                XCTAssertEqual(bounds.x, parent.x + edited.frame.x * parent.width / 1000)
                XCTAssertEqual(bounds.y, parent.y + edited.frame.y * parent.height / 1000)
                XCTAssertEqual(bounds.width, edited.frame.width * parent.width / 1000)
                XCTAssertEqual(bounds.height, edited.frame.height * parent.height / 1000)
                XCTAssertNotEqual(baseline.pixels, changed.pixels)
                let allowed = CGRect(x: before.x, y: before.y, width: before.width, height: before.height)
                    .union(CGRect(x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height))
                let first = [UInt8](baseline.pixels), second = [UInt8](changed.pixels)
                var escaped = 0
                for offset in stride(from: 0, to: first.count, by: 4)
                    where first[offset..<(offset + 4)] != second[offset..<(offset + 4)] {
                    let pixel = offset / 4
                    if !allowed.contains(CGPoint(x: pixel % width, y: pixel / width)) { escaped += 1 }
                }
                XCTAssertEqual(escaped, 0)
                store.updateSelected { $0.hidden = true }
                let hidden = try await runner.render(NativePreviewSnapshot.capture(store, width: width, height: height))
                store.updateSelected { $0.hidden = false; $0.opacity = 0 }
                let transparent = try await runner.render(NativePreviewSnapshot.capture(store, width: width, height: height))
                XCTAssertEqual(hidden.pixels, transparent.pixels)
                XCTAssertNotEqual(hidden.pixels, changed.pixels)
                if width == 1280 { try saveProof(changed, name: "native-settings-child-\(role.rawValue)") }
                store.updateSelected { $0 = child }
            }
        }
    }

    // ------------------------=
    // FUNC: testSettingsTimeoutSliderUsesNativeSizingAndThemePixels
    // DESC: Verifies track and thumb can be independently styled and resized while their preview geometry remains owned by the real timeout value binding.
    // ------------------=
    @MainActor func testSettingsTimeoutSliderUsesNativeSizingAndThemePixels() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 5)
        let thumb = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsSliderThumb })
        let track = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsSliderTrack })
        store.selectElement(thumb.id)
        XCTAssertEqual(store.nativePreviewSimulation.expandedRow, 3)
        let runner = NativePreviewRunner()
        let baseline = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        store.selectElement(track.id)
        store.updateSelected {
            $0.frame = CanvasRect(x: 400, y: 600, width: 520, height: 100)
            $0.fill = StudioColor(red: 40, green: 100, blue: 220, alpha: 210)
            $0.surfaceMaterial = .glass
        }
        store.selectElement(thumb.id)
        store.updateSelected {
            $0.frame = CanvasRect(x: 0, y: 530, width: 100, height: 250)
            $0.fill = StudioColor(red: 195, green: 130, blue: 240, alpha: 255)
            $0.surfaceCorners = .rounded; $0.cornerRadius = 12
        }
        let changed = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        let resolved = try XCTUnwrap(changed.bounds.first { $0.id == thumb.id })
        let parent = try XCTUnwrap(resolved.parentBounds)
        XCTAssertEqual(resolved.x, parent.x)
        XCTAssertEqual(resolved.width, parent.width / 10)
        XCTAssertEqual(resolved.y, parent.y + parent.height * 530 / 1000)
        XCTAssertNotEqual(baseline.pixels, changed.pixels)
        let snapshot = try NativePreviewSnapshot.capture(store, width: 1280, height: 800)
        let saved = try RuntimeTemplateCodec.decode(snapshot.template, collection: .systemSettings)
        XCTAssertEqual(saved.screens[4].elements.first { $0.id == thumb.id }?.frame.x, 0)
        try saveProof(changed, name: "native-settings-timeout-authored")
    }

    // ------------------------=
    // FUNC: testConfigurationButtonCaptionColorAndAlphaStayIndependentFromSurface
    // DESC: Verifies configuration caption RGB and alpha through native glyph pixels while preserving the button surface and all pixels outside its authored bounds.
    // ------------------=
    @MainActor func testConfigurationButtonCaptionColorAndAlphaStayIndependentFromSurface() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: 1)
        store.nativePreviewSimulation.enabled = false
        store.nativePreviewState = .normal
        let index = try XCTUnwrap(store.configurationDocument.screens[0].elements.firstIndex { $0.role == .primaryButton })
        store.configurationDocument.screens[0].elements[index].text = "CAPTION"
        store.configurationDocument.screens[0].elements[index].captionColor = .white
        let element = store.configurationDocument.screens[0].elements[index]
        XCTAssertFalse(element.hidden)
        XCTAssertGreaterThan(element.opacity, 0)
        let runner = NativePreviewRunner()
        let white = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        store.configurationDocument.screens[0].elements[index].captionColor = StudioColor(red: 255, green: 0, blue: 0, alpha: 255)
        let red = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        let whitePixels = [UInt8](white.pixels), redPixels = [UInt8](red.pixels)
        let left = element.frame.x * white.width / 1000, top = element.frame.y * white.height / 1000
        let right = left + element.frame.width * white.width / 1000
        let bottom = top + element.frame.height * white.height / 1000
        var changedPixels = 0, escapedPixels = 0, invalidChannels = 0
        for offset in stride(from: 0, to: whitePixels.count, by: 4) {
            guard whitePixels[offset..<(offset + 4)] != redPixels[offset..<(offset + 4)] else { continue }
            changedPixels += 1
            let pixel = offset / 4, x = pixel % white.width, y = pixel / white.width
            if x < left || x >= right || y < top || y >= bottom { escapedPixels += 1 }
            if redPixels[offset] != whitePixels[offset]
                || redPixels[offset + 1] > whitePixels[offset + 1]
                || redPixels[offset + 2] > whitePixels[offset + 2]
                || redPixels[offset + 3] != whitePixels[offset + 3] { invalidChannels += 1 }
        }
        XCTAssertGreaterThan(changedPixels, 20)
        XCTAssertEqual(escapedPixels, 0)
        XCTAssertEqual(invalidChannels, 0)
        store.configurationDocument.screens[0].elements[index].captionColor = StudioColor(red: 255, green: 0, blue: 0, alpha: 0)
        let transparent = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        store.configurationDocument.screens[0].elements[index].captionColor = .white
        store.configurationDocument.screens[0].elements[index].text = ""
        let empty = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertEqual(transparent.pixels, empty.pixels)
        XCTAssertNotEqual(white.pixels, transparent.pixels)
        try saveProof(red, name: "native-configuration-caption-red")
    }

    // ------------------------=
    // FUNC: testUnsimulatedSettingsDoesNotPaintDetailActionsInGlobalCanvas
    // DESC: Confirms detail-local Settings actions never leak into the generic authoring canvas when native row simulation is disabled.
    // ------------------=
    @MainActor func testUnsimulatedSettingsDoesNotPaintDetailActionsInGlobalCanvas() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 4)
        store.nativePreviewSimulation.enabled = false
        let actions = try XCTUnwrap(store.selectedScreen).elements.filter { $0.role == .settingsRowAction }
        XCTAssertFalse(actions.isEmpty)
        XCTAssertTrue(actions.allSatisfy { !$0.hidden && $0.opacity > 0 })
        let runner = NativePreviewRunner()
        let snapshot = try NativePreviewSnapshot.capture(store, width: 1280, height: 800)
        XCTAssertFalse(snapshot.simulation.enabled)
        let baseline = try await runner.render(snapshot)
        for screen in store.settingsDocument.screens.indices {
            for element in store.settingsDocument.screens[screen].elements.indices
                where store.settingsDocument.screens[screen].elements[element].role == .settingsRowAction {
                store.settingsDocument.screens[screen].elements[element].hidden = true
            }
        }
        let hidden = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertEqual(baseline.pixels, hidden.pixels)
    }

    // ------------------------=
    // FUNC: testNativeActionSelectionRevealsLastExpandedAIRow
    // DESC: Uses production scroll metadata to reveal the final AI action at two real viewport sizes without replaying native layout mathematics in Studio.
    // ------------------=
    @MainActor func testNativeActionSelectionRevealsLastExpandedAIRow() async throws {
        let store = TemplateStore(); store.projectRoot = root; store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 4)
        let action = try XCTUnwrap(store.selectedScreen?.elements.last { $0.role == .settingsRowAction })
        let runner = NativePreviewRunner()
        for viewport in [NativePreviewViewport.compact, .fullHD] {
            store.canvasViewport = viewport
            store.nativePreviewSimulation.scroll = 10000
            store.selectElement(action.id)
            let before = try await runner.render(NativePreviewSnapshot.capture(store))
            let geometry = try XCTUnwrap(before.bounds.first { $0.id == action.id })
            let target = try XCTUnwrap(geometry.revealScroll)
            XCTAssertGreaterThan(target, 0)
            XCTAssertLessThanOrEqual(target, try XCTUnwrap(geometry.maximumScroll))
            XCTAssertTrue(store.applyNativeSelectionReveal(before.bounds, screen: 4, expandedRow: 7))
            XCTAssertEqual(store.nativePreviewSimulation.scroll, target)
            let revealed = try await runner.render(NativePreviewSnapshot.capture(store))
            let bounds = try XCTUnwrap(revealed.bounds.first { $0.id == action.id })
            let viewportBounds = try XCTUnwrap(bounds.viewportBounds)
            XCTAssertGreaterThanOrEqual(bounds.x, viewportBounds.x)
            XCTAssertGreaterThanOrEqual(bounds.y, viewportBounds.y)
            XCTAssertLessThanOrEqual(bounds.x + bounds.width, viewportBounds.x + viewportBounds.width)
            XCTAssertLessThanOrEqual(bounds.y + bounds.height, viewportBounds.y + viewportBounds.height)
            store.nativePreviewSimulation.scroll = 0
            XCTAssertFalse(store.applyNativeSelectionReveal(revealed.bounds, screen: 4, expandedRow: 7))
            XCTAssertEqual(store.nativePreviewSimulation.scroll, 0)
        }
    }

    // ------------------------=
    // FUNC: testSettingsActionNativePixelsFollowUnsavedDetailGeometryAndStyle
    // DESC: Exercises the production renderer with unsaved action frames and styles, validating exported detail-local geometry and collapsed visibility.
    // ------------------=
    @MainActor func testSettingsActionNativePixelsFollowUnsavedDetailGeometryAndStyle() async throws {
        let store = TemplateStore(); store.projectRoot = root; store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 1)
        let action = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsRowAction })
        store.selectElement(action.id)
        let runner = NativePreviewRunner()
        let first = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        let firstBounds = try XCTUnwrap(first.bounds.first { $0.id == action.id })
        let parent = try XCTUnwrap(firstBounds.parentBounds)
        XCTAssertEqual(firstBounds.x, parent.x + action.frame.x * parent.width / 1000)
        XCTAssertEqual(firstBounds.y, parent.y + action.frame.y * parent.height / 1000)
        XCTAssertEqual(firstBounds.width, action.frame.width * parent.width / 1000)
        XCTAssertEqual(firstBounds.height, action.frame.height * parent.height / 1000)
        var focusedSnapshot = try NativePreviewSnapshot.capture(store, width: 1280, height: 800)
        focusedSnapshot.state = .focused
        let focused = try await runner.render(focusedSnapshot)
        focusedSnapshot.state = .pressed
        let pressed = try await runner.render(focusedSnapshot)
        XCTAssertGreaterThan(zip(first.pixels, focused.pixels).filter { $0 != $1 }.count, 100)
        XCTAssertGreaterThan(zip(focused.pixels, pressed.pixels).filter { $0 != $1 }.count, 100)
        XCTAssertEqual(focused.bounds, first.bounds)
        XCTAssertEqual(pressed.bounds, first.bounds)
        store.updateSelected("Move native action") {
            $0.frame = CanvasRect(x: 480, y: 540, width: 460, height: 320)
            $0.actionCaptionMode = .literal; $0.text = "Rename device"
            $0.surfaceMaterial = .matte; $0.surfaceCorners = .rounded; $0.cornerRadius = 14
            $0.fill = StudioColor(red: 42, green: 28, blue: 94, alpha: 230)
            $0.border = StudioColor(red: 186, green: 144, blue: 255, alpha: 255)
        }
        let moved = try XCTUnwrap(store.selectedElement)
        let changed = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        let bounds = try XCTUnwrap(changed.bounds.first { $0.id == action.id })
        let changedParent = try XCTUnwrap(bounds.parentBounds)
        XCTAssertEqual(bounds.x, changedParent.x + moved.frame.x * changedParent.width / 1000)
        XCTAssertEqual(bounds.y, changedParent.y + moved.frame.y * changedParent.height / 1000)
        XCTAssertEqual(bounds.width, moved.frame.width * changedParent.width / 1000)
        XCTAssertEqual(bounds.height, moved.frame.height * changedParent.height / 1000)
        XCTAssertGreaterThan(zip(first.pixels, changed.pixels).filter { $0 != $1 }.count, 500)
        try saveProof(changed, name: "native-settings-action-unsaved-detail-style")
        let wide = try await runner.render(NativePreviewSnapshot.capture(store, width: 1920, height: 1080))
        let wideBounds = try XCTUnwrap(wide.bounds.first { $0.id == action.id })
        let wideParent = try XCTUnwrap(wideBounds.parentBounds)
        XCTAssertEqual(wideBounds.width, moved.frame.width * wideParent.width / 1000)
        XCTAssertEqual(wideBounds.height, moved.frame.height * wideParent.height / 1000)
        store.nativePreviewSimulation.expandedRow = -1
        let collapsed = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertFalse(collapsed.bounds.contains { $0.id == action.id })
        store.nativePreviewSimulation.expandedRow = 0
        store.updateSelected { $0.hidden = true }
        let hidden = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertEqual(hidden.bounds.first { $0.id == action.id }, bounds)
        XCTAssertGreaterThan(zip(hidden.pixels, changed.pixels).filter { $0 != $1 }.count, 500)
        store.updateSelected { $0.hidden = false; $0.opacity = 0 }
        let transparent = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertEqual(transparent.pixels, hidden.pixels)
        store.updateSelected {
            $0.opacity = 100
            $0.captionColor = StudioColor(red: 255, green: 80, blue: 35, alpha: 180)
        }
        let caption = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertEqual(caption.bounds.first { $0.id == action.id }, bounds)
        let captionDifferences = zip(caption.pixels, changed.pixels).filter { $0 != $1 }.count
        XCTAssertGreaterThan(captionDifferences, 30)
        XCTAssertLessThan(captionDifferences, bounds.width * bounds.height * 4)
        try saveProof(caption, name: "native-settings-action-caption-color")
    }

    // ------------------------=
    // FUNC: testAddedWizardScreensRenderThroughNativePipeline
    // DESC: Adds screens through Studio's public editing operations and verifies native frame production beyond the Settings-only eleven-screen limit.
    // ------------------=
    @MainActor func testAddedWizardScreensRenderThroughNativePipeline() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.document = .factoryDefault()
        store.configurationDocument = .factoryConfiguration()
        store.nativePreviewSimulation.enabled = false
        let runner = NativePreviewRunner()
        for collection in [ScreenCollection.installation, .configuration] {
            store.selectScreenCollection(collection, screen: 1)
            for targetCount in [12, 32] {
                while store.activeScreens.count < targetCount {
                    store.selectScreenCollection(collection, screen: store.activeScreens.count)
                    store.addScreen()
                }
                XCTAssertEqual(store.activeScreens.count, targetCount)
                XCTAssertEqual(store.selectedScreenID, targetCount)
                let snapshot = try NativePreviewSnapshot.capture(store)
                let frame = try await runner.render(snapshot)
                XCTAssertEqual(frame.pixels.count, frame.width * frame.height * 4)
                XCTAssertGreaterThan(Set(frame.pixels).count, 16)
                try saveProof(frame, name: "native-\(snapshot.kind)-added-screen-\(targetCount)")
            }
            XCTAssertFalse(store.canAddScreen)
        }
    }

    // ------------------------=
    // FUNC: testSimulationControlRangesMatchNativeSections
    // DESC: Rejects nonexistent dashboard pages and resolves the date component to the active live field.
    // ------------------=
    func testSimulationControlRangesMatchNativeSections() {
        XCTAssertEqual(NativePreviewSimulation.settingsRange(screen: 7, rows: 0), 0...6)
        XCTAssertEqual(NativePreviewSimulation.settingsRange(screen: 8, rows: 0), 0...4)
        XCTAssertEqual(NativePreviewSimulation.settingsRange(screen: 4, rows: 7), -1...6)
        var value = NativePreviewSimulation(); value.expandedRow = 7
        XCTAssertEqual(value.resolved(collection: .systemSettings, screen: 8, rows: 0).expandedRow, 4)
        value.dateFocus = 3; value.datePart = 0
        XCTAssertEqual(value.resolved(collection: .installation, screen: 5, rows: 0).datePart, 3)
        value.dateFocus = 2; value.datePart = 4
        XCTAssertEqual(value.resolved(collection: .installation, screen: 5, rows: 0).datePart, 2)
    }
    private var root: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    // ------------------------=
    // FUNC: snapshot
    // DESC: Creates a bounded immutable request for renderer lifecycle tests independent of source fixtures.
    // ------------------=
    private func snapshot(screen: Int) -> NativePreviewSnapshot {
        NativePreviewSnapshot(projectRoot: root, template: Data(), kind: "installer", screen: screen,
            width: 320, height: 240, atlas: Data(), glyphs: Data(), styles: Data(), background: Data(), assets: [])
    }

    // ------------------------=
    // FUNC: testNewestNativeFrameOwnsCanvas
    // DESC: Verifies an older asynchronous completion cannot overwrite a newer edit's verified framebuffer.
    // ------------------=
    @MainActor func testNewestNativeFrameOwnsCanvas() async {
        let runner = ControlledNativeRenderer()
        let subject = NativePreviewService(runner: runner)
        let first = Task { await subject.update(snapshot(screen: 1)) }
        await runner.wait(for: 1)
        let second = Task { await subject.update(snapshot(screen: 2)) }
        await runner.wait(for: 2)
        await runner.finish(2, value: 222)
        await second.value
        XCTAssertEqual(subject.phase, .ready)
        XCTAssertEqual(subject.frame?.pixels.first, 222)
        await runner.finish(1, value: 111)
        await first.value
        XCTAssertEqual(subject.frame?.pixels.first, 222)
        XCTAssertEqual(subject.phase, .ready)
    }

    // ------------------------=
    // FUNC: testMalformedAndCancelledFramesNeverBecomeCurrent
    // DESC: Retains the last verified pixels with an explicit failure state and rejects a cancelled request's late completion.
    // ------------------=
    @MainActor func testMalformedAndCancelledFramesNeverBecomeCurrent() async {
        let runner = ControlledNativeRenderer()
        let subject = NativePreviewService(runner: runner)
        let valid = Task { await subject.update(snapshot(screen: 1)) }
        await runner.wait(for: 1); await runner.finish(1, value: 90); await valid.value
        let invalid = Task { await subject.update(snapshot(screen: 2)) }
        await runner.wait(for: 2); await runner.finish(2, value: 12, malformed: true); await invalid.value
        XCTAssertEqual(subject.phase, .failed)
        XCTAssertNotNil(subject.errorMessage)
        XCTAssertEqual(subject.frame?.pixels.first, 90)
        let cancelled = Task { await subject.update(snapshot(screen: 3)) }
        await runner.wait(for: 3); subject.cancel(); await runner.finish(3, value: 1); await cancelled.value
        XCTAssertEqual(subject.phase, .idle)
        XCTAssertEqual(subject.frame?.pixels.first, 90)
    }

    // ------------------------=
    // FUNC: testNativeBitmapPreservesOrientationAndTransparency
    // DESC: Checks actual encoded V4 pixels and alpha rather than relying on a host screenshot's appearance.
    // ------------------=
    @MainActor func testNativeBitmapPreservesOrientationAndTransparency() throws {
        let bytes = Data([255,0,0,255, 0,255,0,255, 0,0,255,255, 0,0,0,0])
        let provider = try XCTUnwrap(CGDataProvider(data: bytes as CFData))
        let image = try XCTUnwrap(CGImage(width: 2, height: 2, bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: 8, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue), provider: provider,
            decode: nil, shouldInterpolate: false, intent: .defaultIntent))
        let bmp = try NativePreviewSnapshot.bitmap(NSImage(cgImage: image, size: NSSize(width: 2, height: 2)))
        XCTAssertEqual(bmp.count, 138)
        XCTAssertEqual(Array(bmp[122..<130]), [0,0,255,255, 0,255,0,255])
        XCTAssertEqual(Array(bmp[130..<138]), [255,0,0,255, 0,0,0,0])
        XCTAssertEqual(Array(bmp[22..<26]), [254,255,255,255])
        XCTAssertTrue(NativeBitmapEncoder.supportedBitmap(bmp))
    }

    // ------------------------=
    // FUNC: testUploadedPNGPixelsMatchUIKitPackaging
    // DESC: Compares native preview bytes against the actual Python packaging encoder over low-alpha colors and top-down image rows.
    // ------------------=
    @MainActor func testUploadedPNGPixelsMatchUIKitPackaging() throws {
        let directory = root.appending(path: "builds/.studio-work/behavior-tests")
        let expected = try Data(contentsOf: directory.appending(path: "studio-native-upload.bmp"))
        let actual = try NativeBitmapEncoder.read(directory.appending(path: "studio-native-upload.png"))
        XCTAssertEqual(actual, expected)
        let png = try Data(contentsOf: directory.appending(path: "studio-native-upload.png"))
        let rgba = try UIKitStore(projectRoot: nil).rawRGBA(png, width: 10, height: 10)
        XCTAssertEqual(rgba, try Data(contentsOf: directory.appending(path: "studio-native-upload.rgba")))
    }

    // ------------------------=
    // FUNC: testNativeBitmapPackagingParityAndPathConfinement
    // DESC: Verifies runtime packaging preserves exact native bitmap bytes and refuses traversal or symlink image sources.
    // ------------------=
    func testNativeBitmapPackagingParityAndPathConfinement() throws {
        let directory = root.appending(path: "builds/.studio-work/tmp/native-bitmap-test-\(UUID().uuidString)")
        let assets = directory.appending(path: "assets/boot")
        try FileManager.default.createDirectory(at: assets, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let original = root.appending(path: "assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp")
        let bytes = try Data(contentsOf: original)
        XCTAssertTrue(NativeBitmapEncoder.supportedBitmap(bytes))
        let image = assets.appending(path: "fixture.bmp")
        try bytes.write(to: image)
        XCTAssertEqual(try NativeBitmapEncoder.read(image), bytes)
        var document = InstallerStudioDocument.factoryDefault()
        let index = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.kind == .image })
        document.screens[0].elements[index].imageAsset = "fixture.bmp"
        let encoded = try RuntimeTemplateCodec.encode(document, assetRoot: directory)
        XCTAssertNotNil(encoded.range(of: bytes))
        for path in ["../../fixture.bmp", original.path] {
            document.screens[0].elements[index].imageAsset = path
            XCTAssertThrowsError(try RuntimeTemplateCodec.encode(document, assetRoot: directory))
        }
        let link = assets.appending(path: "outside.bmp")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: original)
        document.screens[0].elements[index].imageAsset = "outside.bmp"
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(document, assetRoot: directory))
    }

    // ------------------------=
    // FUNC: testRendererTimeoutLeavesEditingServiceUsable
    // DESC: Exercises termination of an owned stalled helper and verifies cancellation returns its editing service to an idle state.
    // ------------------=
    @MainActor func testRendererTimeoutLeavesEditingServiceUsable() async throws {
        let directory = root.appending(path: "builds/.studio-work/tmp/native-timeout-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let executable = directory.appending(path: "stalled-renderer")
        try Data("#!/bin/sh\nexec /bin/sleep 30\n".utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: executable.path)
        let service = NativePreviewService(runner: NativePreviewRunner(executableOverride: executable, timeout: .milliseconds(250)))
        let start = ContinuousClock.now
        await service.update(snapshot(screen: 1))
        XCTAssertEqual(service.phase, .failed)
        XCTAssertNil(service.frame)
        XCTAssertNotNil(service.errorMessage)
        XCTAssertLessThan(start.duration(to: .now), .seconds(3))
        service.cancel()
        XCTAssertEqual(service.phase, .idle)
    }

    // ------------------------=
    // FUNC: testProductionRendererUsesUnsavedTextAndStyles
    // DESC: Exercises all native scene kinds and verifies unsaved text and material edits change pixels without changing saved source documents.
    // ------------------=
    @MainActor func testProductionRendererUsesUnsavedTextAndStyles() async throws {
        let store = TemplateStore()
        store.nativePreviewSimulation.enabled = false
        store.projectRoot = root
        let savedURL = root.appending(path: "assets/boot/configuration-screens.infinityui")
        let saved = try Data(contentsOf: savedURL)
        let runner = NativePreviewRunner()
        for collection in ScreenCollection.allCases {
            store.selectScreenCollection(collection, screen: 1)
            let snapshot = try NativePreviewSnapshot.capture(store)
            let first = try await runner.render(snapshot)
            XCTAssertEqual(first.width, 1600)
            XCTAssertEqual(first.height, 1000)
            XCTAssertEqual(first.pixels.count, 6_400_000)
            let element = try XCTUnwrap(store.selectedScreen?.elements.first { ($0.kind == .text || $0.kind == .progressBar) && !$0.hidden && !$0.locked && !$0.text.isEmpty })
            store.selectElement(element.id)
            store.updateSelected("Native preview text edit") { $0.text = "Native production text preview" }
            let edited = try NativePreviewSnapshot.capture(store)
            XCTAssertNotEqual(edited.template, snapshot.template)
            let second = try await runner.render(edited)
            XCTAssertNotEqual(second.pixels, first.pixels)
            try saveProof(first, name: "native-\(snapshot.kind)-baseline")
            try saveProof(second, name: "native-\(snapshot.kind)-unsaved-text")
        }
        store.selectScreenCollection(.configuration, screen: 1)
        let baseline = try await runner.render(NativePreviewSnapshot.capture(store))
        store.uiKit.setSurfaceStyle(2, style: UIKitSurfaceStyle(material: .matte, corners: .rounded, radius: 24))
        let styled = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(styled.pixels, baseline.pixels)
        try saveProof(styled, name: "native-configuration-unsaved-material")
        store.nativePreviewState = .focused
        let focused = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(focused.pixels, styled.pixels)
        store.nativePreviewState = .pressed
        let pressed = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(pressed.pixels, focused.pixels)
        store.canvasViewport = .fullHD
        let wide = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertEqual(wide.width, 1920)
        XCTAssertEqual(wide.height, 1080)
        try saveProof(wide, name: "native-configuration-fullhd-pressed")
        XCTAssertEqual(try Data(contentsOf: savedURL), saved)
    }

    // ------------------------=
    // FUNC: testNativeServiceStatesAreBoundedAndVisiblyRendered
    // DESC: Exercises native clock-field substitutions, destructive dialog and progress pixels without invoking any real system service.
    // ------------------=
    @MainActor func testNativeServiceStatesAreBoundedAndVisiblyRendered() async throws {
        let store = TemplateStore(); store.projectRoot = root
        store.nativePreviewSimulation.enabled = false
        let runner = NativePreviewRunner()
        store.selectScreenCollection(.installation, screen: 5)
        let authored = try await runner.render(NativePreviewSnapshot.capture(store))
        store.nativePreviewSimulation.enabled = true
        let clock = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(clock.pixels, authored.pixels)
        store.nativePreviewSimulation.timeZone = 10
        let japan = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(japan.pixels, clock.pixels)
        try saveProof(japan, name: "native-installer-live-date-time")
        let map = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .timeZoneMap })
        store.selectElement(map.id); store.updateSelected("Map transparency proof") { $0.opacity = 0 }
        let noMap = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertGreaterThan(zip(japan.pixels, noMap.pixels).filter { $0 != $1 }.count, 10_000)
        store.updateSelected("Restore map proof") { $0.opacity = 100 }
        store.selectScreenCollection(.installation, screen: 7)
        let review = try await runner.render(NativePreviewSnapshot.capture(store))
        store.nativePreviewSimulation.confirmation = true
        let confirmation = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(review.pixels, confirmation.pixels)
        try saveProof(confirmation, name: "native-installer-erase-confirmation")
        store.selectScreenCollection(.installation, screen: 8)
        store.nativePreviewSimulation.progress = 10
        let progress = try await runner.render(NativePreviewSnapshot.capture(store))
        store.nativePreviewSimulation.progress = 73
        let later = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(progress.pixels, later.pixels)
        try saveProof(later, name: "native-installer-progress")
        var invalid = store.nativePreviewSimulation; invalid.progress = 101
        XCTAssertThrowsError(try invalid.arguments())
        invalid.progress = 0; invalid.timeZone = 12
        XCTAssertThrowsError(try invalid.arguments())
        store.selectScreenCollection(.systemSettings, screen: 1)
        let general = try await runner.render(NativePreviewSnapshot.capture(store))
        try saveProof(general, name: "native-settings-simulated-general")
        store.selectScreenCollection(.systemSettings, screen: 4)
        let settings = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertFalse(settings.bounds.isEmpty)
        let ids = Set(store.selectedScreen?.elements.map(\.id) ?? [])
        XCTAssertTrue(settings.bounds.allSatisfy { ids.contains($0.id) })
        store.nativePreviewSimulation.expandedRow = 2
        store.nativePreviewSimulation.scroll = 40
        let expanded = try await runner.render(NativePreviewSnapshot.capture(store))
        XCTAssertNotEqual(settings.pixels, expanded.pixels)
        XCTAssertNotEqual(settings.bounds, expanded.bounds)
        try saveProof(expanded, name: "native-settings-simulated-expanded")
        let service = NativePreviewService(runner: runner)
        await service.update(try NativePreviewSnapshot.capture(store))
        XCTAssertEqual(service.frame?.bounds, expanded.bounds)
        for screen in [7,8] {
            store.selectScreenCollection(.systemSettings, screen: screen)
            store.nativePreviewSimulation.expandedRow = 0
            store.nativePreviewSimulation.scroll = 0
            let dashboard = try await runner.render(NativePreviewSnapshot.capture(store))
            XCTAssertFalse(dashboard.bounds.isEmpty)
            store.nativePreviewSimulation.expandedRow = 1
            let page = try await runner.render(NativePreviewSnapshot.capture(store))
            XCTAssertNotEqual(page.pixels, dashboard.pixels)
            try saveProof(dashboard, name: screen == 7 ? "native-settings-network" : "native-settings-nodes")
            if screen == 8 {
                store.nativePreviewSimulation.expandedRow = 0
                let artwork = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsArtwork })
                store.selectElement(artwork.id); store.updateSelected("Hide topology proof") { $0.hidden = true }
                let hidden = try await runner.render(NativePreviewSnapshot.capture(store))
                XCTAssertGreaterThan(zip(hidden.pixels, dashboard.pixels).filter { $0 != $1 }.count, 10_000)
            }
        }
    }

    // ------------------------=
    // FUNC: testNativeSettingsGestureTransformUpdatesAuthoredGeometry
    // DESC: Verifies native framebuffer drag and resize distances update the source template through the exact inverse console transform.
    // ------------------=
    @MainActor func testNativeSettingsGestureTransformUpdatesAuthoredGeometry() throws {
        let store = TemplateStore(); store.selectScreenCollection(.systemSettings, screen: 1)
        store.snapEnabled = false
        let element = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsRowLabel })
        let console = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .console })
        let window = NativePreviewBounds(id: console.id, x: 40, y: 70, width: console.frame.width * 2, height: console.frame.height * 2)
        let scale = NativeCanvasMapping.editingScale(element: element, console: console.frame, window: window, viewport: .studio, scale: 0.5)
        XCTAssertEqual(scale, CGSize(width: 1, height: 1))
        store.selectElement(element.id); store.beginGesture(elementID: element.id)
        store.moveSelected(translation: CGSize(width: 13, height: 9), canvasScale: scale); store.endGesture()
        let moved = try XCTUnwrap(store.selectedElement)
        XCTAssertEqual(moved.frame.x, element.frame.x + 13); XCTAssertEqual(moved.frame.y, element.frame.y + 9)
        store.beginGesture(elementID: element.id)
        store.resizeSelected(handle: .bottomRight, translation: CGSize(width: 18, height: 7), canvasScale: scale); store.endGesture()
        XCTAssertEqual(store.selectedElement?.frame.width, moved.frame.width + 18)
        XCTAssertEqual(store.selectedElement?.frame.height, moved.frame.height + 7)
        store.selectScreenCollection(.systemSettings, screen: 7)
        let card = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsMainCard })
        XCTAssertFalse(store.geometryIsEditable(card))
        store.selectElement(card.id)
        XCTAssertFalse(store.canReorderSelectedLayer)
        store.beginGesture(); store.moveSelected(translation: CGSize(width: 20, height: 20), canvasScale: CGSize(width: 1, height: 1)); store.endGesture()
        store.nudgeSelected(dx: 1, dy: 1, byGrid: false); store.moveLayer(1)
        XCTAssertEqual(store.selectedElement?.frame, card.frame)
        XCTAssertEqual(store.selectedElement?.zIndex, card.zIndex)
    }

    // ------------------------=
    // FUNC: saveProof
    // DESC: Exports the exact native framebuffer for visual acceptance when the build kit requests proof artifacts.
    // ------------------=
    @MainActor private func saveProof(_ frame: NativePreviewFrame, name: String) throws {
        guard let directory = ProcessInfo.processInfo.environment["INFINITY_STUDIO_PRECISION_PROOFS"] else { return }
        let image = try XCTUnwrap(frame.image())
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        let destination = URL(fileURLWithPath: directory)
        try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)
        try png.write(to: destination.appending(path: name + ".png"), options: .atomic)
    }
}
