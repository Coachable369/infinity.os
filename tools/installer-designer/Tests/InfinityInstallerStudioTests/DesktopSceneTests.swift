import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class DesktopSceneTests: XCTestCase {
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
        for scene in 1...3 {
            store.selectScreenCollection(.desktop,screen:scene)
            let frame=try await runner.render(NativePreviewSnapshot.capture(store,width:1536,height:1024))
            let image=try XCTUnwrap(frame.image())
            let png=try XCTUnwrap(NSBitmapImageRep(cgImage:image).representation(using:.png,properties:[:]))
            try png.write(to:try XCTUnwrap(store.projectRoot).appending(path:"builds/studio-desktop-\(scene).png"))
            let i=try XCTUnwrap(store.desktopDocument.screens[scene-1].elements.firstIndex {$0.role == .console})
            store.desktopDocument.screens[scene-1].elements[i].opacity=20
            let changed=try await runner.render(NativePreviewSnapshot.capture(store,width:1536,height:1024))
            XCTAssertNotEqual(frame.pixels,changed.pixels)
        }
    }
}
