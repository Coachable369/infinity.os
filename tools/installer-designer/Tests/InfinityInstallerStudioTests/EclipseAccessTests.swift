import XCTest
import SwiftUI
import CoreText
@testable import InfinityInstallerStudio

final class EclipseAccessTests: XCTestCase {
    // ------------------------=
    // FUNC: testConfigurationUsesPackagedInterWithNativeWeights
    // DESC: Verifies the resolved font asset, weight axis and scalable glyph advances instead of accepting a named host-font fallback.
    // ------------------=
    @MainActor func testConfigurationUsesPackagedInterWithNativeWeights() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let asset = root.appending(path: "assets/fonts/Inter-Variable.ttf").standardizedFileURL
        for (strong, weight) in [(false, 400), (true, 600)] {
            let font = try XCTUnwrap(InfinityUIKit.EclipseAccess.nativeFont(size: 24, semibold: strong, projectRoot: root))
            let resolved = try XCTUnwrap(CTFontCopyAttribute(font, kCTFontURLAttribute) as? URL).standardizedFileURL
            XCTAssertEqual(resolved, asset)
            let variations = try XCTUnwrap(CTFontCopyVariation(font) as? [NSNumber: NSNumber])
            let axes = try XCTUnwrap(CTFontCopyVariationAxes(font) as? [[CFString: Any]])
            let weightDefault = try XCTUnwrap(axes.first {
                ($0[kCTFontVariationAxisIdentifierKey] as? NSNumber)?.intValue == 0x77676874
            }?[kCTFontVariationAxisDefaultValueKey] as? NSNumber)
            XCTAssertEqual((variations[NSNumber(value: 0x77676874)] ?? weightDefault).intValue, weight)
            let larger = try XCTUnwrap(InfinityUIKit.EclipseAccess.nativeFont(size: 48, semibold: strong, projectRoot: root))
            var characters: [UniChar] = [73, 110, 102, 105, 110, 105, 116, 121]
            var glyphs = [CGGlyph](repeating: 0, count: characters.count)
            XCTAssertTrue(CTFontGetGlyphsForCharacters(font, &characters, &glyphs, characters.count))
            let width = CTFontGetAdvancesForGlyphs(font, .horizontal, &glyphs, nil, glyphs.count)
            let largeWidth = CTFontGetAdvancesForGlyphs(larger, .horizontal, &glyphs, nil, glyphs.count)
            XCTAssertGreaterThan(width, 60)
            XCTAssertEqual(largeWidth, width * 2, accuracy: 0.01)
        }
        XCTAssertNil(InfinityUIKit.EclipseAccess.nativeFont(size: 24, semibold: false, projectRoot: nil))
    }

    // ------------------------=
    // FUNC: testNetworkGlyphsUseExactNativeAtlasPixels
    // DESC: Checks preview glyph bytes against their native packaged RGBA cells and rejects out-of-range indices.
    // ------------------=
    func testNetworkGlyphsUseExactNativeAtlasPixels() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let atlas = try Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba"))
        for index in [5, 0, 8] {
            let image = try XCTUnwrap(InfinityUIKit.EclipseAccess.glyph(index, projectRoot: root))
            let cgImage = try XCTUnwrap(image.cgImage(forProposedRect: nil, context: nil, hints: nil))
            XCTAssertEqual(cgImage.width, 64)
            XCTAssertEqual(cgImage.height, 64)
            let pixels = try XCTUnwrap(cgImage.dataProvider?.data) as Data
            XCTAssertEqual(pixels, atlas.subdata(in: index * 16_384..<(index + 1) * 16_384))
        }
        XCTAssertNil(InfinityUIKit.EclipseAccess.glyph(-1, projectRoot: root))
        XCTAssertNil(InfinityUIKit.EclipseAccess.glyph(16, projectRoot: root))
    }

    // ------------------------=
    // FUNC: testAsymmetricCropUsesTopOriginRasterCoordinates
    // DESC: Verifies the actual cropped pixels against colored source quadrants, independently of NSImage drawing coordinates.
    // ------------------=
    func testAsymmetricCropUsesTopOriginRasterCoordinates() throws {
        let bitmap = try XCTUnwrap(NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 100, pixelsHigh: 100,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 400, bitsPerPixel: 32))
        let pixels = try XCTUnwrap(bitmap.bitmapData)
        for y in 0..<100 {
            for x in 0..<100 {
                let offset = (y * 100 + x) * 4
                pixels[offset] = (y < 50 && x < 50) || (y >= 50 && x >= 50) ? 255 : 0
                pixels[offset + 1] = x >= 50 ? 255 : 0
                pixels[offset + 2] = y >= 50 ? 255 : 0
                pixels[offset + 3] = 255
            }
        }
        let image = NSImage(cgImage: try XCTUnwrap(bitmap.cgImage), size: NSSize(width: 100, height: 100))
        let cropped = try XCTUnwrap(ImageCrop(left: 10, top: 10, right: 40, bottom: 60).previewImage(image))
        let result = NSBitmapImageRep(cgImage: try XCTUnwrap(cropped.cgImage(forProposedRect: nil, context: nil, hints: nil)))
        XCTAssertEqual(result.pixelsWide, 50)
        XCTAssertEqual(result.pixelsHigh, 30)
        for y in [0, 29] {
            let red = try XCTUnwrap(result.colorAt(x: 0, y: y)?.usingColorSpace(.deviceRGB))
            let green = try XCTUnwrap(result.colorAt(x: 49, y: y)?.usingColorSpace(.deviceRGB))
            XCTAssertEqual(red.redComponent, 1, accuracy: 0.01)
            XCTAssertEqual(red.blueComponent, 0, accuracy: 0.01)
            XCTAssertEqual(green.greenComponent, 1, accuracy: 0.01)
            XCTAssertEqual(green.redComponent, 0, accuracy: 0.01)
        }
    }

    // ------------------------=
    // FUNC: testAuthoredWordmarkCropContainsVisibleSourcePixels
    // DESC: Decodes the original BMP and checks that the persisted top crop retains substantial visible branding instead of transparent lower rows.
    // ------------------=
    func testAuthoredWordmarkCropContainsVisibleSourcePixels() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let source = try XCTUnwrap(NSImage(contentsOf: root.appending(path: "assets/boot/infinity-eclipse-header-v1.bmp")))
        let cropped = try XCTUnwrap(ImageCrop(left: 14, top: 1, right: 12, bottom: 80).previewImage(source))
        let bitmap = NSBitmapImageRep(cgImage: try XCTUnwrap(cropped.cgImage(forProposedRect: nil, context: nil, hints: nil)))
        var visible = 0
        for y in 0..<bitmap.pixelsHigh {
            for x in 0..<bitmap.pixelsWide {
                if let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB),
                   color.alphaComponent > 0.2,
                   max(color.redComponent, color.greenComponent, color.blueComponent) > 0.2 { visible += 1 }
            }
        }
        XCTAssertGreaterThan(visible, 5_000)
    }

    // ------------------------=
    // FUNC: testAuthoredMaterialMatchesNativeSemantics
    // DESC: Verifies that template controls preserve authored alpha and map to the shared native atlas for every collection.
    // ------------------=
    func testAuthoredMaterialMatchesNativeSemantics() {
        let palette = InfinityUIKit.EclipseAccess.self
        let idle = InfinityUIKit.precisionMaterial(state: .idle, fill: palette.primary.withAlpha(117), border: palette.border)
        XCTAssertEqual(idle.fill, palette.primary.withAlpha(117))
        XCTAssertEqual(idle.opacity, 117)
        XCTAssertEqual(idle.edgeOpacity, 255)
        XCTAssertEqual(idle.focus, 0)
        let focus = InfinityUIKit.precisionMaterial(state: .focused, fill: palette.primary, border: palette.border)
        XCTAssertEqual(focus.focus, 255)
        XCTAssertEqual(focus.edge, palette.border)
        let pressed = InfinityUIKit.precisionMaterial(state: .pressed, fill: palette.primary, border: palette.border)
        XCTAssertTrue(pressed.pressed)
        for configuration in [false, true] {
            XCTAssertEqual(InfinityUIKit.precisionKind(role: .primaryButton, kind: .button, configuration: configuration), .primary)
            XCTAssertEqual(InfinityUIKit.precisionKind(role: .backButton, kind: .button, configuration: configuration), .secondary)
            XCTAssertEqual(InfinityUIKit.precisionKind(role: .input, configuration: configuration), .field)
        }
    }

    // ------------------------=
    // FUNC: testTransparentRuntimeContentRemainsInvisibleWhenSelected
    // DESC: Ensures selecting an invisible semantic content bound cannot introduce a focus border or fill.
    // ------------------=
    @MainActor func testTransparentRuntimeContentRemainsInvisibleWhenSelected() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let material = InfinityUIKit.precisionMaterial(state: .focused,
            fill: InfinityUIKit.EclipseAccess.surface.withAlpha(0), border: InfinityUIKit.EclipseAccess.mutedBorder.withAlpha(0))
        let image = try XCTUnwrap(ImageRenderer(content: PrecisionSurface(kind: .panel, material: material, projectRoot: root)
            .frame(width: 100, height: 50)).cgImage)
        let bitmap = NSBitmapImageRep(cgImage: image)
        for point in [(0, 25), (2, 25), (50, 25)] {
            XCTAssertEqual(try XCTUnwrap(bitmap.colorAt(x: point.0, y: point.1)).alphaComponent, 0)
        }
    }

    // ------------------------=
    // FUNC: testConfigurationProgressUsesCircularAuthoredFootprint
    // DESC: Renders the actual canvas element and checks its occupied pixels instead of accepting a progress-label snapshot.
    // ------------------=
    @MainActor func testConfigurationProgressUsesCircularAuthoredFootprint() throws {
        let store = TemplateStore()
        store.selectScreenCollection(.configuration, screen: 1)
        store.selectElement(nil)
        var element = StudioElement.make(name: "Progress", kind: .panel, role: .progressSegment,
            frame: CanvasRect(x: 0, y: 0, width: 80, height: 40), zIndex: 1)
        element.fill = InfinityUIKit.EclipseAccess.primary
        element.border = InfinityUIKit.EclipseAccess.border
        let image = try XCTUnwrap(ImageRenderer(content: CanvasElementView(element: element, store: store,
            canvasScale: CGSize(width: 1, height: 1)).frame(width: 80, height: 40)).cgImage)
        let bitmap = NSBitmapImageRep(cgImage: image)
        XCTAssertEqual(InfinityUIKit.EclipseAccess.progressDiameter(CGSize(width: 80, height: 40)), 40)
        XCTAssertGreaterThan(try XCTUnwrap(bitmap.colorAt(x: 40, y: 20)).alphaComponent, 0.95)
        XCTAssertLessThan(try XCTUnwrap(bitmap.colorAt(x: 0, y: 0)).alphaComponent, 0.05)
        XCTAssertEqual(try XCTUnwrap(bitmap.colorAt(x: 70, y: 20)).alphaComponent, 0)
    }

    // ------------------------=
    // FUNC: testFactoryBuiltInArtworkIsReferencedWithoutReencoding
    // DESC: Exercises runtime encoding with actual assets and verifies the alpha-sensitive built-in images are not embedded as converted payloads.
    // ------------------=
    func testFactoryBuiltInArtworkIsReferencedWithoutReencoding() throws {
        let document = InstallerStudioDocument.factoryConfiguration()
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let encoded = try RuntimeTemplateCodec.encode(document, assetRoot: root)
        XCTAssertEqual(encoded, try RuntimeTemplateCodec.encode(document))
        XCTAssertEqual(try RuntimeTemplateCodec.decode(encoded, collection: .configuration), document)
        for screen in document.screens.prefix(8) {
            let images = screen.elements.filter { $0.kind == .image }
            XCTAssertEqual(images.count, 2)
            let wordmark = try XCTUnwrap(images.first { $0.role == .image })
            XCTAssertEqual(wordmark.crop, ImageCrop(left: 14, top: 1, right: 12, bottom: 80))
            XCTAssertEqual(wordmark.frame, CanvasRect(x: 510, y: 400, width: 440, height: 100))
            for image in images { XCTAssertTrue(FileManager.default.fileExists(atPath: root.appending(path: image.imageAsset).path)) }
        }
    }

    // ------------------------=
    // FUNC: testCaptureAuthoredConfiguration
    // DESC: Optionally captures the actual saved Studio configuration layers for comparison with installed screenshots; it does not modify the project.
    // ------------------=
    @MainActor func testCaptureAuthoredConfiguration() throws {
        guard let directory = ProcessInfo.processInfo.environment["INFINITY_ECLIPSE_PROOFS"] else { return }
        let destination = URL(fileURLWithPath: directory).standardizedFileURL
        guard destination.path.hasPrefix("/opt/codebase/infinity.os/builds/validation/") else {
            XCTFail("Canvas evidence must remain in repository-local validation storage")
            return
        }
        let store = TemplateStore()
        try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)
        for screen in 1...8 {
            store.selectScreenCollection(.configuration, screen: screen)
            store.selectElement(nil)
            let elements = try XCTUnwrap(store.selectedScreen?.elements)
            let view = ZStack(alignment: .topLeading) {
                InfinityUIKit.EclipseAccess.background.color
                ForEach(elements.filter { !$0.hidden }.sorted { $0.zIndex < $1.zIndex }) { element in
                    CanvasElementView(element: element, store: store, canvasScale: CGSize(width: 1.6, height: 1))
                }
                if screen == 7, let content = elements.first(where: { $0.role == .content && !$0.hidden }) {
                    ConfigurationNetworkPreview(canvasScale: CGSize(width: 1.6, height: 1),
                        content: content.frame, projectRoot: store.projectRoot)
                }
            }.frame(width: 1600, height: 1000)
            let image = try XCTUnwrap(ImageRenderer(content: view).cgImage)
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: destination.appending(path: "configuration-\(screen).png"))
        }
    }
}
