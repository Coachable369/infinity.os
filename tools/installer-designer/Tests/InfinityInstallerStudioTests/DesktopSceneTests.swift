import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class DesktopSceneTests: XCTestCase {
    // ------------------------=
    // FUNC: testDockMigrationAndPersistence
    // DESC: Preserves custom scenes while introducing one native-bound dock whose image geometry survives binary round trip.
    // ------------------=
    @MainActor func testDockMigrationAndPersistence() throws {
        var source = InstallerStudioDocument.factoryDesktop()
        source.screens.removeAll { $0.elements.contains { $0.role == .desktopDock } }
        var custom = source.screens[1]; custom.id = 4; custom.title = "My Workspace"
        custom.elements.removeAll { $0.role.rawValue >= 49 }
        source.screens.append(custom)
        let migrated = source.migratedForDesktopDock()
        XCTAssertEqual(Array(migrated.screens.prefix(4)), source.screens)
        XCTAssertEqual(migrated.migratedForDesktopDock(), migrated)
        XCTAssertEqual(migrated.screens.last?.elements.filter { (73...80).contains($0.role.rawValue) }.count, 8)
        let binary = try RuntimeTemplateCodec.encode(migrated, collection: .desktop)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(binary, collection: .desktop), migrated)
    }
    // ------------------------=
    // FUNC: testDesktopEditingRoundTrip
    // DESC: Exercises selection, removal, undo and binary persistence of real desktop-bound layers.
    // ------------------=
    @MainActor func testDesktopEditingRoundTrip() throws {
        let store=TemplateStore()
        store.desktopDocument = .factoryDesktop()
        store.selectScreenCollection(.desktop, screen: 1)
        let original=store.desktopDocument
        let item=try XCTUnwrap(store.selectedScreen?.elements.first {$0.role == .desktopNetwork})
        store.selectElement(item.id)
        XCTAssertFalse(store.selectedElementIsRequired)
        store.updateSelected {$0.frame.x=200;$0.hidden=true}
        XCTAssertNotEqual(store.desktopDocument,original)
        store.undo()
        XCTAssertEqual(store.desktopDocument,original)
        store.moveScreen(1)
        XCTAssertEqual(store.desktopDocument,original)
        let bytes=try RuntimeTemplateCodec.encode(original,assetRoot:store.projectRoot,collection:.desktop)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(bytes,collection:.desktop),original)
        XCTAssertEqual(try NativePreviewSnapshot.capture(store,width:1280,height:900).kind,"desktop")
    }
    // ------------------------=
    // FUNC: testDesktopNativePixels
    // DESC: Checks production desktop pixels across all three scenes and after a hidden native navigation control.
    // ------------------=
    @MainActor func testDesktopNativePixels() async throws {
        let store=TemplateStore()
        store.desktopDocument = .factoryDesktop()
        let runner=NativePreviewRunner(timeout:.seconds(60))
        for scene in 1...store.desktopDocument.screens.count {
            store.selectScreenCollection(.desktop,screen:scene)
            let frame=try await runner.render(NativePreviewSnapshot.capture(store,width:1536,height:1024))
            let image=try XCTUnwrap(frame.image())
            let png=try XCTUnwrap(NSBitmapImageRep(cgImage:image).representation(using:.png,properties:[:]))
            try png.write(to:try XCTUnwrap(store.projectRoot).appending(path:"builds/studio-desktop-\(scene).png"))
            let i=try XCTUnwrap(store.desktopDocument.screens[scene-1].elements.firstIndex {$0.role == .console || $0.role == .desktopDock})
            store.desktopDocument.screens[scene-1].elements[i].opacity=20
            let changed=try await runner.render(NativePreviewSnapshot.capture(store,width:1536,height:1024))
            XCTAssertNotEqual(frame.pixels,changed.pixels)
        }
    }
}
